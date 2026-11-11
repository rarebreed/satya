//! Character Sheet Persistence Vault (`SQLite` + `~/Satya/characters/*.satyachar` + Apache Iggy
//! `character-sheets` topic -> PyLance), `.satyachar` Binary Export/Import (`showSaveFilePicker`),
//! Anti-Cheat Verification Badge, and Multimodal Asset Attachments (Portraits, Tokens, PDFs, 3D `.glb`).

use crate::wasm_client::character_sheet::{
    AnatomicalZoneState, BoneFractureStatus, CharacterSheetState, render_character_sheet,
};
use crate::wasm_client::chat_and_ttrpg::append_system_notice;
use crate::wasm_client::dom::{escape_html, fetch_json, get_element, select_value, window};
use crate::wasm_client::state::{with_state, with_state_mut};
use js_sys::{Function, Object, Promise, Reflect, Uint8Array};
use satya_engine::{
    AcuteMentalTraumaTier, ArmorProfileId, HemorrhageGrade, HitLocationId, MeleeWeaponId,
    MoraleState, PhysiologicalStatusCondition, SkillAccessTier, SocialWoundKind,
    aptitudes_from_record, aptitudes_to_record, characteristics_from_record,
    characteristics_to_record, verify_character_sheet_legality,
};
use satya_schema::{
    AnatomicalZoneRecord, BindCharacterSessionRequest, CharacterAssetKind, CharacterEventType,
    CharacterListResponse, CharacterSheetEvent, CharacterSheetRecord, CharacterVerificationStatus,
    ImportCharacterRequest, Participant, SaveCharacterRequest, SaveCharacterResponse,
    SchemaRegistry, SessionMode, SessionRole, UploadCharacterAssetRequest, subjects,
};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};

const B64_CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Pure Rust standard Base64 encoder for `.satyachar` and multimodal asset payloads.
#[must_use]
pub fn encode_base64_bytes(input: &[u8]) -> String {
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = u32::from(chunk[0]);
        let b1 = if chunk.len() > 1 {
            u32::from(chunk[1])
        } else {
            0
        };
        let b2 = if chunk.len() > 2 {
            u32::from(chunk[2])
        } else {
            0
        };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(B64_CHARS[((triple >> 18) & 0x3F) as usize] as char);
        out.push(B64_CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(B64_CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(B64_CHARS[(triple & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

fn bleed_to_i32(grade: HemorrhageGrade) -> i32 {
    match grade {
        HemorrhageGrade::Grade0None => 0,
        HemorrhageGrade::Grade1Capillary => 1,
        HemorrhageGrade::Grade2Venous => 2,
        HemorrhageGrade::Grade3MajorVenousVisceral => 3,
        HemorrhageGrade::Grade4ArterialSeverance => 4,
    }
}

fn i32_to_bleed(val: i32) -> HemorrhageGrade {
    match val {
        1 => HemorrhageGrade::Grade1Capillary,
        2 => HemorrhageGrade::Grade2Venous,
        3 => HemorrhageGrade::Grade3MajorVenousVisceral,
        4 => HemorrhageGrade::Grade4ArterialSeverance,
        _ => HemorrhageGrade::Grade0None,
    }
}

fn cond_to_code(c: PhysiologicalStatusCondition) -> &'static str {
    match c {
        PhysiologicalStatusCondition::DazzledBlurredVision => "DAZZLED",
        PhysiologicalStatusCondition::Blinded => "BLINDED",
        PhysiologicalStatusCondition::Tinnitus => "TINNITUS",
        PhysiologicalStatusCondition::Deafened => "DEAFENED",
        PhysiologicalStatusCondition::StunnedDazed => "STUNNED",
        PhysiologicalStatusCondition::Concussed => "CONCUSSED",
        PhysiologicalStatusCondition::DizzyVertigo => "DIZZY",
        PhysiologicalStatusCondition::NauseaSickened => "NAUSEA",
        PhysiologicalStatusCondition::PinnedImmobilized => "PINNED",
        PhysiologicalStatusCondition::Paralyzed => "PARALYZED",
        PhysiologicalStatusCondition::UnconsciousComatose => "UNCONSCIOUS",
    }
}

fn code_to_cond(code: &str) -> Option<PhysiologicalStatusCondition> {
    match code.trim().to_ascii_uppercase().as_str() {
        "DAZZLED" => Some(PhysiologicalStatusCondition::DazzledBlurredVision),
        "BLINDED" => Some(PhysiologicalStatusCondition::Blinded),
        "TINNITUS" => Some(PhysiologicalStatusCondition::Tinnitus),
        "DEAFENED" => Some(PhysiologicalStatusCondition::Deafened),
        "STUNNED" => Some(PhysiologicalStatusCondition::StunnedDazed),
        "CONCUSSED" => Some(PhysiologicalStatusCondition::Concussed),
        "DIZZY" => Some(PhysiologicalStatusCondition::DizzyVertigo),
        "NAUSEA" => Some(PhysiologicalStatusCondition::NauseaSickened),
        "PINNED" => Some(PhysiologicalStatusCondition::PinnedImmobilized),
        "PARALYZED" => Some(PhysiologicalStatusCondition::Paralyzed),
        "UNCONSCIOUS" => Some(PhysiologicalStatusCondition::UnconsciousComatose),
        _ => None,
    }
}

impl CharacterSheetState {
    /// Converts this live [`CharacterSheetState`] into the canonical [`CharacterSheetRecord`] for
    /// SQLite storage, `.satyachar` binary framing, and Apache Iggy `character-sheets` ingestion.
    #[must_use]
    pub fn to_character_sheet_record(&self, owner_user_id: &str) -> CharacterSheetRecord {
        let hit_zones = self
            .hit_zones
            .iter()
            .map(|z| AnatomicalZoneRecord {
                location_code: format!("ZONE_{}", z.location as usize),
                wound_tier: i32::from(z.wound_tier),
                bone_status: z.bone_status.code().to_string(),
                bleed_grade: bleed_to_i32(z.bleed_grade),
            })
            .collect();

        let mental_trauma = self.mental_trauma.map(|mt| {
            match mt {
                AcuteMentalTraumaTier::Tier1AcuteMentalShockOodaFreeze => "TIER_1",
                AcuteMentalTraumaTier::Tier2PanicFightOrFlight => "TIER_2",
                AcuteMentalTraumaTier::Tier3DissociationShellShock => "TIER_3",
                AcuteMentalTraumaTier::Tier4Catatonia => "TIER_4",
            }
            .to_string()
        });

        let social_wound = self.social_wound.map(|sw| {
            match sw {
                SocialWoundKind::RhetoricalStrain => "STRAIN",
                SocialWoundKind::Doubt => "DOUBT",
                SocialWoundKind::CognitiveDissonance => "DISSONANCE",
                SocialWoundKind::EgoWound => "EGO_WOUND",
            }
            .to_string()
        });

        let mut rec = CharacterSheetRecord {
            character_id: if self.character_id.trim().is_empty() {
                format!("char-{}", self.preset_id)
            } else {
                self.character_id.clone()
            },
            owner_user_id: owner_user_id.to_string(),
            version: 1,
            character_name: self.character_name.clone(),
            callsign_or_role: self.callsign_or_role.clone(),
            age_years: self.age_years as i32,
            origin_label: self.origin_label.clone(),
            campaign_sqm: i32::from(self.campaign_sqm),
            operational_heat: i32::from(self.operational_heat),
            granted_access_tags: self
                .granted_access_tags
                .iter()
                .map(|t| t.as_str().to_string())
                .collect(),
            stage_history: self.stage_history.clone(),
            characteristics: characteristics_to_record(&self.characteristics),
            aptitudes: aptitudes_to_record(&self.aptitudes),
            skills: self
                .skills
                .iter()
                .map(|(k, &v)| (k.clone(), v as i32))
                .collect(),
            equipped_ranged_ids: self.equipped_ranged_ids.clone(),
            equipped_melee_codes: self
                .equipped_melee
                .iter()
                .map(|&m| format!("{}", m as usize))
                .collect(),
            equipped_armor_codes: self
                .equipped_armor
                .iter()
                .map(|&a| format!("{}", a as usize))
                .collect(),
            field_gear_ids: self.field_gear.iter().map(|g| g.id.clone()).collect(),
            hit_zones,
            fatigue_level: self.fatigue_level,
            exertion_marks: self.exertion_marks,
            wound_shock_index: self.wound_shock_index,
            blood_units_lost: self.blood_units_lost,
            active_status_conditions: self
                .active_status_conditions
                .iter()
                .map(|&c| cond_to_code(c).to_string())
                .collect(),
            morale_state: self.morale_state.as_str().to_string(),
            mental_trauma,
            social_composure_lost: self.social_composure_lost,
            social_wound,
            psyche_drift_marks: self.psyche_drift_marks,
            notes: Some(self.notes.clone()),
            lifepath_config_json: None,
            assets: self.assets.clone(),
            verification_status: self.verification_status,
            sp_budget_valid: true,
            dag_prereqs_valid: true,
            audit_violations: self.audit_violations.clone(),
            content_sha256: String::new(),
            server_signature_hex: None,
            updated_at_us: 0,
        };
        let audit = verify_character_sheet_legality(&rec);
        rec.sp_budget_valid = audit.sp_budget_valid;
        rec.dag_prereqs_valid = audit.dag_prereqs_valid;
        rec.audit_violations = audit.violations;
        rec.verification_status = if !audit.is_legal {
            CharacterVerificationStatus::TamperedOrOverBudget
        } else if rec.server_signature_hex.is_some() {
            CharacterVerificationStatus::VerifiedOfficial
        } else {
            CharacterVerificationStatus::LegalCustom
        };
        rec
    }

    /// Hydrates this [`CharacterSheetState`] from an imported or vault-loaded [`CharacterSheetRecord`].
    pub fn apply_character_sheet_record(&mut self, rec: &CharacterSheetRecord) {
        self.character_id = rec.character_id.clone();
        self.character_name = rec.character_name.clone();
        self.callsign_or_role = rec.callsign_or_role.clone();
        self.age_years = rec.age_years.max(1) as u32;
        self.origin_label = rec.origin_label.clone();
        self.campaign_sqm = rec.campaign_sqm.clamp(0, 10) as u8;
        self.operational_heat = rec.operational_heat.clamp(0, 10) as u8;
        self.granted_access_tags = rec
            .granted_access_tags
            .iter()
            .filter_map(|c| SkillAccessTier::from_code_str(c))
            .collect();
        self.stage_history = rec.stage_history.clone();
        self.characteristics = characteristics_from_record(&rec.characteristics);
        self.aptitudes = aptitudes_from_record(&rec.aptitudes);
        self.skills = rec
            .skills
            .iter()
            .map(|(k, &v)| (k.clone(), v.max(0) as usize))
            .collect();
        self.equipped_ranged_ids = rec.equipped_ranged_ids.clone();
        self.equipped_melee = rec
            .equipped_melee_codes
            .iter()
            .filter_map(|c| {
                c.parse::<usize>()
                    .ok()
                    .and_then(|idx| MeleeWeaponId::ALL.get(idx).copied())
            })
            .collect();
        self.equipped_armor = rec
            .equipped_armor_codes
            .iter()
            .filter_map(|c| {
                c.parse::<usize>()
                    .ok()
                    .and_then(|idx| ArmorProfileId::ALL.get(idx).copied())
            })
            .collect();
        let gear_refs: Vec<&str> = rec.field_gear_ids.iter().map(String::as_str).collect();
        self.field_gear = Self::gear_from_ids(&gear_refs);
        self.notes = rec.notes.clone().unwrap_or_default();
        self.verification_status = rec.verification_status;
        self.audit_violations = rec.audit_violations.clone();
        self.assets = rec.assets.clone();

        self.fatigue_level = rec.fatigue_level.clamp(0, 5);
        self.exertion_marks = rec.exertion_marks.max(0);
        self.wound_shock_index = rec.wound_shock_index.clamp(0, 5);
        self.blood_units_lost = rec.blood_units_lost.clamp(0.0, 10.0);
        if let Some(m) = MoraleState::from_code_str(&rec.morale_state) {
            self.morale_state = m;
        }
        self.mental_trauma = rec.mental_trauma.as_deref().and_then(|c| match c {
            "TIER_1" => Some(AcuteMentalTraumaTier::Tier1AcuteMentalShockOodaFreeze),
            "TIER_2" => Some(AcuteMentalTraumaTier::Tier2PanicFightOrFlight),
            "TIER_3" => Some(AcuteMentalTraumaTier::Tier3DissociationShellShock),
            "TIER_4" => Some(AcuteMentalTraumaTier::Tier4Catatonia),
            _ => None,
        });
        self.social_composure_lost = rec.social_composure_lost.max(0);
        self.social_wound = rec.social_wound.as_deref().and_then(|c| match c {
            "STRAIN" => Some(SocialWoundKind::RhetoricalStrain),
            "DOUBT" => Some(SocialWoundKind::Doubt),
            "DISSONANCE" => Some(SocialWoundKind::CognitiveDissonance),
            "EGO_WOUND" => Some(SocialWoundKind::EgoWound),
            _ => None,
        });
        self.psyche_drift_marks = rec.psyche_drift_marks.clamp(0, 10);
        self.active_status_conditions = rec
            .active_status_conditions
            .iter()
            .filter_map(|c| code_to_cond(c))
            .collect();

        self.hit_zones = HitLocationId::ALL.map(AnatomicalZoneState::new_healthy);
        for (idx, z_rec) in rec.hit_zones.iter().enumerate() {
            let target_idx = z_rec
                .location_code
                .strip_prefix("ZONE_")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(idx);
            if let Some(zone) = self.hit_zones.get_mut(target_idx) {
                zone.wound_tier = z_rec.wound_tier.clamp(0, 5) as u8;
                zone.bone_status = BoneFractureStatus::from_code(&z_rec.bone_status);
                zone.bleed_grade = i32_to_bleed(z_rec.bleed_grade);
            }
        }
    }
}

/// Formats the Anti-Cheat Verification Badge HTML (`VERIFIED_OFFICIAL`, `LEGAL_CUSTOM`, `TAMPERED_OR_OVER_BUDGET`).
#[must_use]
pub fn format_verification_badge_html(sheet: &CharacterSheetState) -> String {
    match sheet.verification_status {
        CharacterVerificationStatus::VerifiedOfficial => {
            r#"<span id="char-sheet-verification-badge" class="pill pill-active" title="Server HMAC-SHA256 verified and all Satya Engine budgets/DAG gates passed">Verified Official · Satya Signed</span>"#.to_string()
        }
        CharacterVerificationStatus::LegalCustom => {
            r#"<span id="char-sheet-verification-badge" class="pill pill-iggy" title="All Satya Engine Characteristic, Aptitude, SP Ledger, and Skill DAG gates passed">Legal Custom · Engine Audited</span>"#.to_string()
        }
        CharacterVerificationStatus::TamperedOrOverBudget => {
            let reason = if sheet.audit_violations.is_empty() {
                "Failed anti-cheat audit".to_string()
            } else {
                escape_html(&sheet.audit_violations.join(" | "))
            };
            format!(
                r#"<span id="char-sheet-verification-badge" class="pill pill-danger" title="{reason}">Tampered / Over-Budget</span>"#
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

pub(crate) async fn read_first_file_bytes(input_id: &str) -> Option<(String, String, Vec<u8>)> {
    let el = get_element(input_id)?;
    let files = Reflect::get(&el, &JsValue::from_str("files")).ok()?;
    let file = Reflect::get(&files, &JsValue::from_f64(0.0)).ok()?;
    if file.is_undefined() || file.is_null() {
        return None;
    }
    let name = Reflect::get(&file, &JsValue::from_str("name"))
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_else(|| "character.satyachar".to_string());
    let mime = Reflect::get(&file, &JsValue::from_str("type"))
        .ok()
        .and_then(|v| v.as_string())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "application/octet-stream".to_string());
    let buf_fn = Reflect::get(&file, &JsValue::from_str("arrayBuffer"))
        .ok()?
        .dyn_into::<Function>()
        .ok()?;
    let promise = buf_fn.call0(&file).ok()?.dyn_into::<Promise>().ok()?;
    let buf_val = JsFuture::from(promise).await.ok()?;
    let bytes = Uint8Array::new(&buf_val).to_vec();
    Some((name, mime, bytes))
}

pub(crate) async fn prompt_save_satyachar_bytes(suggested_name: &str, bytes: &[u8]) -> bool {
    let Some(win) = window() else {
        return false;
    };
    let u8_arr = Uint8Array::from(bytes);

    if let Ok(picker_val) = Reflect::get(&win, &JsValue::from_str("showSaveFilePicker"))
        && let Ok(picker_fn) = picker_val.dyn_into::<Function>()
    {
        let opts = Object::new();
        let _ = Reflect::set(
            &opts,
            &JsValue::from_str("suggestedName"),
            &JsValue::from_str(suggested_name),
        );
        if let Ok(handle_prom_val) = picker_fn.call1(&win, &opts)
            && let Ok(handle_prom) = handle_prom_val.dyn_into::<Promise>()
            && let Ok(handle) = JsFuture::from(handle_prom).await
            && let Ok(create_w_val) = Reflect::get(&handle, &JsValue::from_str("createWritable"))
            && let Ok(create_w_fn) = create_w_val.dyn_into::<Function>()
            && let Ok(w_prom_val) = create_w_fn.call0(&handle)
            && let Ok(w_prom) = w_prom_val.dyn_into::<Promise>()
            && let Ok(writable) = JsFuture::from(w_prom).await
        {
            if let Ok(write_fn_val) = Reflect::get(&writable, &JsValue::from_str("write"))
                && let Ok(write_fn) = write_fn_val.dyn_into::<Function>()
                && let Ok(wp) = write_fn.call1(&writable, &u8_arr)
                && let Ok(wp_prom) = wp.dyn_into::<Promise>()
            {
                let _ = JsFuture::from(wp_prom).await;
            }
            if let Ok(close_fn_val) = Reflect::get(&writable, &JsValue::from_str("close"))
                && let Ok(close_fn) = close_fn_val.dyn_into::<Function>()
                && let Ok(cp) = close_fn.call0(&writable)
                && let Ok(cp_prom) = cp.dyn_into::<Promise>()
            {
                let _ = JsFuture::from(cp_prom).await;
            }
            return true;
        }
    }

    if let Some(doc) = win.document()
        && let Ok(a_el) = doc.create_element("a")
    {
        let data_url = format!(
            "data:application/octet-stream;base64,{}",
            encode_base64_bytes(bytes)
        );
        let _ = a_el.set_attribute("href", &data_url);
        let _ = a_el.set_attribute("download", suggested_name);
        if let Ok(html_a) = a_el.dyn_into::<web_sys::HtmlElement>() {
            html_a.click();
            return true;
        }
    }
    false
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
                    "Log in first (or click a Demo User in the top bar) to save to the Character Vault and ~/Satya/characters/.".to_string();
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

        let slug: String = record
            .character_name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '_'
                }
            })
            .collect::<String>()
            .trim_matches('_')
            .to_string();
        let filename = format!(
            "{}.satyachar",
            if slug.is_empty() { "operator" } else { &slug }
        );

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
            let registry = SchemaRegistry::new_with_builtin_schemas().expect("builtin schemas");
            let sid = session_id.unwrap_or_else(|| "local-export".to_string());
            let Ok(metadata) =
                registry.build_metadata(1, subjects::CHARACTER_SHEET, 1, &sid, SessionMode::Ttrpg)
            else {
                return Vec::new();
            };
            let mut actor = Participant::human(
                &record.owner_user_id,
                &record.owner_user_id,
                &record.character_name,
                SessionRole::Player,
            );
            actor.character_id = Some(record.character_id.clone());
            let event = CharacterSheetEvent {
                metadata,
                character_event_type: CharacterEventType::CharacterExported,
                actor,
                character: record,
            };
            registry
                .encode_framed(subjects::CHARACTER_SHEET, 1, &event)
                .unwrap_or_default()
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

        if let Ok(registry) = SchemaRegistry::new_with_builtin_schemas()
            && let Ok((_, event)) = registry.decode_framed::<CharacterSheetEvent>(&bytes, None)
        {
            let mut rec = event.character;
            let report = verify_character_sheet_legality(&rec);
            rec.sp_budget_valid = report.sp_budget_valid;
            rec.dag_prereqs_valid = report.dag_prereqs_valid;
            rec.audit_violations = report.violations;
            rec.verification_status = if report.is_legal {
                CharacterVerificationStatus::LegalCustom
            } else {
                CharacterVerificationStatus::TamperedOrOverBudget
            };
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
/// to `POST /api/characters/{id}/assets` and logs `CHARACTER_ASSET_UPLOADED` to Apache Iggy.
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
