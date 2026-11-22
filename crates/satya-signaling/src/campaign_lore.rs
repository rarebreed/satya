//! Active campaign pack resolution (`awakened` default vs `space_opera` optional),
//! TOML/Markdown campaign lore omnisearch, and starter NPC profile loading for `satya-signaling`.

use crate::characters::audit_and_stamp_character_record;
use crate::gm_api::verify_gm_service_header;
use crate::npcs::{
    SpoilerShieldQuery, can_caller_view_gm_secrets_with_shield, compute_apparent_condition,
    load_session_npcs_for_omnisearch,
};
use crate::state::SignalingAppState;
use axum::{
    Json,
    extract::{Path as AxumPath, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use chrono::Utc;
use satya_campaign::{
    NpcGenerationParams, NpcTemplateKind, generate_npc_from_prompt_with_pack,
};
use satya_engine::{CharacteristicCode, PrerequisiteMode, lookup_skill_node};
use satya_maps::SectorPreset;
use satya_rules_book::{extract_markdown_headings, render_rules_markdown_to_html, slugify_heading};
use satya_schema::{
    CampaignLoreCatalogResponse, CampaignLoreChapterDocument, CampaignLoreChapterSummary,
    CharacterCharacteristicsRecord, CharacterSheetRecord, GmLookupHit, GmLookupSourceKind,
    SessionNpcEntry,
};
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Access capabilities passed to [`append_omnisearch_lore_and_npc_hits`].
#[derive(Debug, Clone, Copy, Default)]
pub struct OmnisearchAccess {
    pub can_see_secrets: bool,
    pub can_manage: bool,
}

impl OmnisearchAccess {
    /// Read-only player-safe access (`can_see_secrets = false`, `can_manage = false`).
    #[must_use]
    pub const fn player() -> Self {
        Self {
            can_see_secrets: false,
            can_manage: false,
        }
    }
}

fn fnv1a64(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash.max(1)
}

/// Resolved campaign-pack defaults loaded from `SATYA_CAMPAIGN_PACK`, `config/ai.toml`,
/// `config/campaign.toml`, or [`SignalingAppState::campaign_pack`].
#[derive(Debug, Clone)]
pub struct ResolvedPackDefaults {
    pub pack_name: String,
    pub default_sector_preset: SectorPreset,
    pub hostile_token_label: String,
    pub ally_token_label: String,
    pub default_ai_player_name: String,
    pub lore_dirs: Vec<String>,
    pub npc_toml_file: String,
}

/// Resolves the repository root containing `config/campaign.toml`, `docs/`, and `data/`.
pub fn workspace_root() -> PathBuf {
    if Path::new("config/campaign.toml").exists() || Path::new("docs/ttrpg").exists() {
        return PathBuf::from(".");
    }
    let manifest_parent = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    if manifest_parent.join("config/campaign.toml").exists()
        || manifest_parent.join("docs/ttrpg").exists()
    {
        return manifest_parent;
    }
    PathBuf::from(".")
}

fn read_toml_table(path: &Path) -> Option<toml::Table> {
    let raw = std::fs::read_to_string(path).ok()?;
    raw.parse::<toml::Table>().ok()
}

fn load_campaign_config(root: &Path) -> satya_campaign::CampaignConfigFile {
    satya_campaign::load_campaign_config_from_path(&root.join("config/campaign.toml"))
        .unwrap_or_else(|| satya_campaign::embedded_campaign_config().clone())
}

/// Resolves the active campaign pack name via:
/// `SATYA_CAMPAIGN_PACK` env > non-default `explicit_pack` (`state.campaign_pack`) >
/// `config/ai.toml` `[ai_gm].campaign_pack` > `config/campaign.toml` `active_pack` > `"awakened"`.
pub fn resolve_active_pack_name(explicit_pack: Option<&str>) -> String {
    let default_pack = satya_campaign::default_active_pack_name();
    if let Ok(env_val) = std::env::var("SATYA_CAMPAIGN_PACK") {
        let trimmed = env_val.trim().to_ascii_lowercase();
        if !trimmed.is_empty() {
            return trimmed;
        }
    }
    if let Some(exp) = explicit_pack {
        let trimmed = exp.trim().to_ascii_lowercase();
        if !trimmed.is_empty() && trimmed != default_pack {
            return trimmed;
        }
    }
    let root = workspace_root();
    if let Some(ai_tbl) = read_toml_table(&root.join("config/ai.toml"))
        && let Some(ai_pack) = ai_tbl
            .get("ai_gm")
            .and_then(|v| v.as_table())
            .and_then(|t| t.get("campaign_pack"))
            .and_then(|v| v.as_str())
    {
        let trimmed = ai_pack.trim().to_ascii_lowercase();
        if !trimmed.is_empty() {
            return trimmed;
        }
    }
    let cfg = load_campaign_config(&root);
    let active = cfg.active_pack.trim().to_ascii_lowercase();
    if !active.is_empty() {
        return active;
    }
    if let Some(exp) = explicit_pack {
        let trimmed = exp.trim().to_ascii_lowercase();
        if !trimmed.is_empty() {
            return trimmed;
        }
    }
    default_pack.to_string()
}

/// Resolves full pack defaults (sector preset fallback, token labels, lore dirs, NPC TOML)
/// directly from `config/campaign.toml` and `satya_campaign::embedded_campaign_config()`.
pub fn resolve_pack_defaults(explicit_pack: Option<&str>) -> ResolvedPackDefaults {
    let pack_name = resolve_active_pack_name(explicit_pack);
    let root = workspace_root();
    let cfg = load_campaign_config(&root);
    let embedded = satya_campaign::embedded_campaign_config();
    let pack = cfg
        .packs
        .get(&pack_name)
        .or_else(|| embedded.packs.get(&pack_name))
        .or_else(|| cfg.packs.get(cfg.active_pack.as_str()))
        .or_else(|| satya_campaign::embedded_pack_spec(&pack_name))
        .expect("embedded campaign config defines at least one pack");

    let ai_name = pack.default_ai_player.character_name.trim();
    let default_ai_player_name = if ai_name.is_empty() {
        "AI Player".to_string()
    } else if ai_name.ends_with("(AI Player)") {
        ai_name.to_string()
    } else {
        format!("{ai_name} (AI Player)")
    };

    ResolvedPackDefaults {
        pack_name,
        default_sector_preset: SectorPreset::from_code_str(&pack.default_sector_preset)
            .unwrap_or(SectorPreset::CommunityClinicAnnex),
        hostile_token_label: pack.default_hostile_token.label.clone(),
        ally_token_label: pack.default_ally_token.label.clone(),
        default_ai_player_name,
        lore_dirs: pack.lore_dirs.clone(),
        npc_toml_file: pack.npc_toml_file.clone(),
    }
}

fn toml_npc_to_session_entry(entry: &toml::Table, is_gm: bool) -> Option<SessionNpcEntry> {
    let npc_id = entry
        .get("npc_id")
        .and_then(|v| v.as_str())?
        .trim()
        .to_ascii_lowercase();
    if npc_id.is_empty() {
        return None;
    }
    let name = entry
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or(&npc_id)
        .to_string();
    let full_archetype = entry
        .get("archetype")
        .and_then(|v| v.as_str())
        .unwrap_or("Contact");
    let archetype = if !is_gm {
        entry
            .get("public_archetype")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(full_archetype)
            .to_string()
    } else {
        full_archetype.to_string()
    };
    let full_faction = entry.get("faction").and_then(|v| v.as_str());
    let faction = if !is_gm {
        entry
            .get("public_faction")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .or(full_faction)
            .map(ToString::to_string)
    } else {
        full_faction.map(ToString::to_string)
    };
    let disposition = entry
        .get("disposition")
        .and_then(|v| v.as_str())
        .map(ToString::to_string);
    let notes = entry.get("notes").and_then(|v| v.as_str()).unwrap_or("");
    let public_summary = entry
        .get("public_summary")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(notes)
        .to_string();
    let age_years = entry
        .get("age_years")
        .and_then(|v| v.as_integer())
        .unwrap_or(36) as i32;
    let fatigue = entry
        .get("fatigue")
        .and_then(|v| v.as_integer())
        .unwrap_or(0);
    let wound_shock = entry
        .get("wound_shock")
        .and_then(|v| v.as_integer())
        .unwrap_or(0);
    let apparent_condition = format!(
        "{}{}",
        disposition.as_deref().unwrap_or("steady"),
        if fatigue > 0 || wound_shock > 0 {
            format!(" · Fat {fatigue}/Shock {wound_shock}")
        } else {
            String::new()
        }
    );

    let mut psyche_summary = Vec::new();
    let mut top_skills = Vec::new();
    let mut gm_secret_dossier = None;
    let mut immediate_agenda = None;
    let mut core_drive = None;
    let mut dual_psyche_wedge_hint = None;

    if is_gm {
        if let Some(psyche_tbl) = entry.get("psyche").and_then(|v| v.as_table()) {
            psyche_summary = psyche_tbl
                .iter()
                .filter_map(|(k, v)| v.as_integer().map(|score| format!("{k}: {score}")))
                .collect();
        }
        if let Some(skills_tbl) = entry.get("skills").and_then(|v| v.as_table()) {
            let mut pairs: Vec<(&str, i64)> = skills_tbl
                .iter()
                .filter_map(|(k, v)| v.as_integer().map(|n| (k.as_str(), n)))
                .collect();
            pairs.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
            top_skills = pairs
                .into_iter()
                .take(5)
                .map(|(k, n)| format!("{k} (N={n})"))
                .collect();
        }
        immediate_agenda = entry
            .get("immediate_agenda")
            .and_then(|v| v.as_str())
            .map(ToString::to_string);
        core_drive = entry
            .get("core_drive")
            .and_then(|v| v.as_str())
            .map(ToString::to_string);
        dual_psyche_wedge_hint = entry
            .get("dual_psyche_wedge_hint")
            .and_then(|v| v.as_str())
            .map(ToString::to_string);
        let secrets: Vec<&str> = entry
            .get("secrets")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();
        let mut dossier_parts = Vec::new();
        if !notes.trim().is_empty() {
            dossier_parts.push(format!("Notes: {}", notes.trim()));
        }
        if !secrets.is_empty() {
            dossier_parts.push(format!("Secrets: {}", secrets.join("; ")));
        }
        if let Some(ag) = &immediate_agenda {
            dossier_parts.push(format!("Immediate Agenda: {ag}"));
        }
        if let Some(cd) = &core_drive {
            dossier_parts.push(format!("Core Drive: {cd}"));
        }
        if let Some(wd) = &dual_psyche_wedge_hint {
            dossier_parts.push(format!("Wedge: {wd}"));
        }
        if !dossier_parts.is_empty() {
            gm_secret_dossier = Some(dossier_parts.join("\n"));
        }
    }

    Some(SessionNpcEntry {
        character_id: npc_id,
        character_name: name,
        callsign_or_role: match &faction {
            Some(fac) if !fac.trim().is_empty() => format!("{archetype} · {fac}"),
            _ => archetype,
        },
        age_years,
        public_summary,
        apparent_condition,
        is_persistent: true,
        gm_character: None,
        psyche_summary,
        top_skills,
        gm_secret_dossier,
        disposition,
        faction_id: faction,
        immediate_agenda,
        core_drive,
        dual_psyche_wedge_hint,
    })
}

/// Loads starter NPC roster entries from the active campaign pack's `npc_toml_file`.
pub fn load_pack_starter_npcs(explicit_pack: Option<&str>, is_gm: bool) -> Vec<SessionNpcEntry> {
    let pack = resolve_pack_defaults(explicit_pack);
    let path = workspace_root().join(&pack.npc_toml_file);
    let Some(tbl) = read_toml_table(&path) else {
        return Vec::new();
    };
    tbl.get("npcs")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_table())
                .filter_map(|entry| toml_npc_to_session_entry(entry, is_gm))
                .collect()
        })
        .unwrap_or_default()
}

/// Finds a raw `[[npcs]]` TOML table in the active campaign pack by `npc_id` or `name` (case-insensitive).
pub fn find_pack_starter_npc_table(
    explicit_pack: Option<&str>,
    npc_id_or_name: &str,
) -> Option<toml::Table> {
    let needle = npc_id_or_name.trim().to_ascii_lowercase();
    if needle.is_empty() {
        return None;
    }
    let pack = resolve_pack_defaults(explicit_pack);
    let path = workspace_root().join(&pack.npc_toml_file);
    let tbl = read_toml_table(&path)?;
    let arr = tbl.get("npcs").and_then(|v| v.as_array())?;

    for entry in arr.iter().filter_map(|v| v.as_table()) {
        let id_match = entry
            .get("npc_id")
            .and_then(|v| v.as_str())
            .is_some_and(|s| s.trim().eq_ignore_ascii_case(&needle));
        if id_match {
            return Some(entry.clone());
        }
    }
    for entry in arr.iter().filter_map(|v| v.as_table()) {
        let name_match = entry
            .get("name")
            .and_then(|v| v.as_str())
            .is_some_and(|s| s.trim().eq_ignore_ascii_case(&needle));
        if name_match {
            return Some(entry.clone());
        }
    }
    let slug_needle = needle.replace([' ', '.', '-', '\''], "_");
    arr.iter()
        .filter_map(|v| v.as_table())
        .find(|entry| {
            let id_low = entry
                .get("npc_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_ascii_lowercase();
            let name_low = entry
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_ascii_lowercase();
            id_low == slug_needle || name_low.contains(&needle)
        })
        .cloned()
}

fn normalize_starter_skill_id(raw: &str) -> &str {
    let trimmed = raw.trim();
    if lookup_skill_node(trimmed).is_some() {
        return trimmed;
    }
    match trimmed {
        "computer_science" => "algorithms_systems_architecture",
        "cryptography" => "cryptography_quantum_computing",
        "close_combat" => "martial_arts_striking",
        "small_arms" => "pistols_handguns",
        "athletics" => "running_sprinting",
        "medicine" => "clinical_diagnosis_internal_medicine",
        "interrogation" => "interrogation_counter_interrogation",
        "surveillance" => "surveillance_reconnaissance_countersurveillance",
        "forensics" => "forensics_crime_scene_investigation",
        "mechanics_improvised" => "automotive_small_engine_mechanics",
        "stealth_infiltration" => "stealth_sneaking_hiding",
        "electronics_repair" => "electrical_power_systems",
        other => other,
    }
}

fn bump_char_min(c: &mut CharacterCharacteristicsRecord, code: CharacteristicCode, min_val: i32) {
    let v = min_val.clamp(1, 20);
    let slot = match code {
        CharacteristicCode::SPD => &mut c.spd,
        CharacteristicCode::FOR => &mut c.for_stat,
        CharacteristicCode::DEX => &mut c.dex,
        CharacteristicCode::KIN => &mut c.kin,
        CharacteristicCode::HLT => &mut c.hlt,
        CharacteristicCode::FIT => &mut c.fit,
        CharacteristicCode::MEM => &mut c.mem,
        CharacteristicCode::ANA => &mut c.ana,
        CharacteristicCode::INS => &mut c.ins,
        CharacteristicCode::CRE => &mut c.cre,
        CharacteristicCode::DIS => &mut c.dis,
        CharacteristicCode::FOC => &mut c.foc,
        CharacteristicCode::COM => &mut c.com,
        CharacteristicCode::PRE => &mut c.pre,
        CharacteristicCode::ELO => &mut c.elo,
        CharacteristicCode::SAV => &mut c.sav,
        CharacteristicCode::BEA => &mut c.bea,
        CharacteristicCode::WRM => &mut c.wrm,
        CharacteristicCode::SIG => &mut c.sig,
        CharacteristicCode::HEA => &mut c.hea,
        CharacteristicCode::TAC => &mut c.tac,
        CharacteristicCode::OLF => &mut c.olf,
        CharacteristicCode::TAS => &mut c.tas,
        CharacteristicCode::ITU => &mut c.itu,
        CharacteristicCode::EGO => &mut c.ego,
        CharacteristicCode::SLF => &mut c.slf,
        CharacteristicCode::OPN => &mut c.opn,
        CharacteristicCode::BLG => &mut c.blg,
        CharacteristicCode::EXT => &mut c.ext,
        CharacteristicCode::CNF => &mut c.cnf,
        CharacteristicCode::AGG => &mut c.agg,
        CharacteristicCode::TRS => &mut c.trs,
        CharacteristicCode::CPS => &mut c.cps,
        _ => return,
    };
    if *slot < v {
        *slot = v;
    }
}

fn ensure_starter_skill_tree(
    ch: &mut CharacterSheetRecord,
    skill_id: &str,
    target_rank: i32,
    depth: usize,
) {
    if depth >= 8 {
        return;
    }
    let Some(spec) = lookup_skill_node(skill_id) else {
        return;
    };
    for &(code, min_val) in spec.min_chars {
        bump_char_min(&mut ch.characteristics, code, min_val);
    }
    if let Some(first_access) = spec.allowed_access.first() {
        let tag = first_access.as_str().to_string();
        if !ch
            .granted_access_tags
            .iter()
            .any(|t| t.eq_ignore_ascii_case(&tag))
        {
            ch.granted_access_tags.push(tag);
        }
    }
    match spec.parent_mode {
        PrerequisiteMode::AllOf => {
            for &(parent_id, req_rank) in spec.parent_prereqs {
                ensure_starter_skill_tree(ch, parent_id, req_rank.clamp(1, 10) as i32, depth + 1);
            }
        }
        PrerequisiteMode::AnyOf => {
            if let Some(&(parent_id, req_rank)) = spec.parent_prereqs.first() {
                ensure_starter_skill_tree(ch, parent_id, req_rank.clamp(1, 10) as i32, depth + 1);
            }
        }
    }
    let clamped = target_rank.clamp(1, 10);
    let slot = ch.skills.entry(spec.id.to_string()).or_insert(0);
    if *slot < clamped {
        *slot = clamped;
    }
}

fn apply_starter_toml_stats(ch: &mut CharacterSheetRecord, entry: &toml::Table) {
    if let Some(attrs) = entry.get("attribute_tns").and_then(|v| v.as_table()) {
        for (k, v) in attrs {
            let Some(raw_val) = v.as_integer() else {
                continue;
            };
            let val = (raw_val as i32).clamp(1, 20);
            match k.trim().to_ascii_uppercase().as_str() {
                "STR" | "FOR" => {
                    ch.characteristics.for_stat = val;
                    ch.characteristics.fit = val;
                }
                "DEX" => {
                    ch.characteristics.dex = val;
                    ch.characteristics.spd = ch.characteristics.spd.max(val - 1);
                    ch.characteristics.kin = ch.characteristics.kin.max(val - 1);
                }
                "INT" | "ANA" => {
                    ch.characteristics.ana = val;
                    ch.characteristics.mem = ch.characteristics.mem.max(val - 1);
                    ch.characteristics.ins = ch.characteristics.ins.max(val - 1);
                }
                "WIL" | "DIS" => {
                    ch.characteristics.dis = val;
                    ch.characteristics.foc = val;
                }
                other => {
                    if let Some(code) = CharacteristicCode::from_code_str(other) {
                        bump_char_min(&mut ch.characteristics, code, val);
                    }
                }
            }
        }
    }
    if let Some(psy) = entry.get("psyche").and_then(|v| v.as_table()) {
        for (k, v) in psy {
            let Some(raw_val) = v.as_integer() else {
                continue;
            };
            let val = (raw_val as i32).clamp(1, 20);
            match k.trim().to_ascii_lowercase().as_str() {
                "opn" => ch.characteristics.opn = val,
                "slf" => ch.characteristics.slf = val,
                "blg" => ch.characteristics.blg = val,
                "agg" => ch.characteristics.agg = val,
                "ego" => ch.characteristics.ego = val,
                "cnf" => ch.characteristics.cnf = val,
                "ext" => ch.characteristics.ext = val,
                "trs" => ch.characteristics.trs = val,
                "cps" => ch.characteristics.cps = val,
                _ => {}
            }
        }
    }
    if let Some(skills_tbl) = entry.get("skills").and_then(|v| v.as_table()) {
        for (k, v) in skills_tbl {
            let Some(raw_rank) = v.as_integer() else {
                continue;
            };
            let canon = normalize_starter_skill_id(k);
            ensure_starter_skill_tree(ch, canon, raw_rank as i32, 0);
        }
    }
}

/// Materializes a canonical universal campaign starter NPC TOML entry into a full
/// rules-legal `SessionNpcEntry` (with audited `VerifiedOfficial` `CharacterSheetRecord`).
pub fn materialize_starter_npc_entry(
    state: &SignalingAppState,
    entry: &toml::Table,
    owner_uid: &str,
    is_persistent: bool,
) -> Option<SessionNpcEntry> {
    let mut npc = toml_npc_to_session_entry(entry, true)?;
    npc.is_persistent = is_persistent;

    let pack = resolve_pack_defaults(Some(state.campaign_pack.as_str()));
    let gen_res = generate_npc_from_prompt_with_pack(
        &NpcGenerationParams {
            prompt: format!(
                "{} — {}. {}",
                npc.character_name, npc.callsign_or_role, npc.public_summary
            ),
            name_override: Some(npc.character_name.clone()),
            template: NpcTemplateKind::Auto,
            faction_archetype: None,
            career_terms: 3,
            seed: fnv1a64(&npc.character_id),
            owner_user_id: owner_uid.to_string(),
        },
        &pack.pack_name,
    );

    let mut ch = gen_res.character;
    ch.character_id = npc.character_id.clone();
    ch.owner_user_id = owner_uid.to_string();
    ch.character_name = npc.character_name.clone();
    ch.callsign_or_role = npc.callsign_or_role.clone();
    ch.age_years = npc.age_years;
    ch.faction_id = npc.faction_id.clone();
    ch.fatigue_level = entry.get("fatigue").and_then(|v| v.as_integer()).unwrap_or(0) as i32;
    ch.wound_shock_index = entry
        .get("wound_shock")
        .and_then(|v| v.as_integer())
        .unwrap_or(0) as i32;
    ch.notes = npc.gm_secret_dossier.clone();
    ch.updated_at_us = Utc::now().timestamp_micros();
    ch.lifepath_config_json = None;

    apply_starter_toml_stats(&mut ch, entry);
    audit_and_stamp_character_record(state, &mut ch, true);
    let cond = compute_apparent_condition(&ch);
    if !cond.trim().is_empty() {
        npc.apparent_condition = cond;
    }
    npc.gm_character = Some(ch);
    Some(npc)
}

fn collect_markdown_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    for p in paths {
        if p.is_dir() {
            collect_markdown_files(&p, out);
        } else if p
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
            && !p
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case("SUMMARY.md"))
        {
            out.push(p);
        }
    }
}

fn score_text_match(query_lower: &str, keywords: &[String], title: &str, body: &str) -> usize {
    if query_lower.is_empty() {
        return 1;
    }
    let title_low = title.to_ascii_lowercase();
    let body_low = body.to_ascii_lowercase();
    let mut score = 0usize;
    if title_low.contains(query_lower) {
        score += 22;
    }
    if body_low.contains(query_lower) {
        score += 12;
    }
    for kw in keywords {
        if title_low.contains(kw) {
            score += 8;
        } else if body_low.contains(kw) {
            score += 4;
        }
    }
    score
}

fn is_public_campaign_lore(rel_label: &str, body: &str) -> bool {
    let rel_low = rel_label.to_ascii_lowercase().replace('\\', "/");
    if rel_low.contains("04-chronicle-and-gm/")
        || rel_low.ends_with("/prologue.md")
        || rel_low.ends_with("satyang_and_four_pillars.md")
        || rel_low.ends_with("summary.md")
    {
        return false;
    }
    let body_low = body.to_ascii_lowercase();
    ![
        "gm reference",
        "secret architect",
        "gm-only",
        "gm ground truth",
        "gm_conspiracy_secret",
        "mirror-hypocris",
        "blue lotus",
        "inner blue lotus",
        "arc2_satyang",
        "shadow-board",
        "conspiracy truth",
        "do not reveal",
    ]
    .iter()
    .any(|marker| body_low.contains(marker))
}

/// Appends ranked [`GmLookupSourceKind::CampaignLore`] (Markdown + TOML) and
/// [`GmLookupSourceKind::SessionNpc`] (SQLite `session_npcs` + pack starter TOML)
/// hits into `hits`, redacting GM secrets unless `access.can_see_secrets` is true.
pub async fn append_omnisearch_lore_and_npc_hits(
    state: &SignalingAppState,
    session_id: &str,
    access: OmnisearchAccess,
    query: &str,
    scope: &str,
    hits: &mut Vec<GmLookupHit>,
) {
    let is_gm = access.can_see_secrets;
    let norm_scope = scope.trim().to_ascii_uppercase();
    let q_low = query.trim().to_ascii_lowercase();
    let keywords: Vec<String> = q_low
        .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
        .filter(|w| w.len() >= 3)
        .map(ToString::to_string)
        .collect();
    let pack = resolve_pack_defaults(Some(state.campaign_pack.as_str()));
    let root = workspace_root();

    let include_lore = matches!(
        norm_scope.as_str(),
        "ALL" | "LORE" | "CAMPAIGN" | "CAMPAIGN_LORE" | "DOCS" | "SETTING"
    );
    let include_npcs = matches!(
        norm_scope.as_str(),
        "ALL"
            | "NPC"
            | "NPCS"
            | "SESSION_NPC"
            | "SESSION_NPCS"
            | "CHARACTER"
            | "CHARACTERS"
            | "LORE"
    );

    if include_lore {
        for rel_dir in &pack.lore_dirs {
            let mut md_files = Vec::new();
            collect_markdown_files(&root.join(rel_dir), &mut md_files);
            for path in md_files {
                let Ok(content) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let rel_label = path
                    .strip_prefix(&root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                for (sec_idx, section) in content.split("\n## ").enumerate() {
                    let body = if sec_idx == 0 {
                        section.trim().to_string()
                    } else {
                        format!("## {}", section.trim())
                    };
                    if body.is_empty() || (!is_gm && !is_public_campaign_lore(&rel_label, &body)) {
                        continue;
                    }
                    let heading = body
                        .lines()
                        .next()
                        .unwrap_or(&rel_label)
                        .trim_start_matches('#')
                        .trim();
                    let score = score_text_match(&q_low, &keywords, heading, &body);
                    if score > 0 {
                        hits.push(GmLookupHit {
                            source_kind: GmLookupSourceKind::CampaignLore,
                            item_id: format!("{rel_label}#{sec_idx}"),
                            title: heading.to_string(),
                            subtitle: format!("Campaign Lore ({}) · {rel_label}", pack.pack_name),
                            snippet: body.chars().take(420).collect(),
                            is_persistent: true,
                            score,
                            section_anchor: Some(slugify_heading(heading)),
                            can_delete: false,
                            is_campaign_default: true,
                        });
                    }
                }
            }
        }
    }

    if include_npcs {
        let mut seen_ids = BTreeSet::new();
        let mut seen_names = BTreeSet::new();
        let db_npcs = load_session_npcs_for_omnisearch(&state.auth, session_id, is_gm).await;
        let db_npc_ids: BTreeSet<String> = db_npcs.iter().map(|n| n.character_id.clone()).collect();
        let db_npc_names: BTreeSet<String> = db_npcs
            .iter()
            .map(|n| n.character_name.trim().to_ascii_lowercase())
            .filter(|n| !n.is_empty())
            .collect();
        hits.retain(|h| {
            if h.source_kind != GmLookupSourceKind::GmMemory {
                return true;
            }
            if let Some(npc_id) = h.item_id.strip_prefix("mem_")
                && db_npc_ids.contains(npc_id)
            {
                return false;
            }
            if h.item_id.starts_with("mem_npc_")
                && let Some(npc_name) =
                    crate::gm_persistence::extract_npc_name_from_memory_title(&h.title)
                && db_npc_names.contains(&npc_name)
            {
                return false;
            }
            true
        });

        for (is_db, npc) in db_npcs.into_iter().map(|n| (true, n)).chain(
            load_pack_starter_npcs(Some(state.campaign_pack.as_str()), is_gm)
                .into_iter()
                .map(|n| (false, n)),
        ) {
            let norm_name = npc.character_name.trim().to_ascii_lowercase();
            if !seen_ids.insert(npc.character_id.clone())
                || (!norm_name.is_empty() && !seen_names.insert(norm_name))
            {
                continue;
            }
            let searchable = format!(
                "{} {} {} {} {} {}",
                npc.callsign_or_role,
                npc.faction_id.as_deref().unwrap_or(""),
                npc.disposition.as_deref().unwrap_or(""),
                npc.public_summary,
                npc.immediate_agenda.as_deref().unwrap_or(""),
                npc.gm_secret_dossier.as_deref().unwrap_or(""),
            );
            let base_score = score_text_match(&q_low, &keywords, &npc.character_name, &searchable);
            if base_score > 0 {
                let score = if is_db { base_score + 24 } else { base_score };
                let kind_prefix = if is_db {
                    "Session NPC".to_string()
                } else {
                    format!("Campaign NPC ({} · Not Loaded)", pack.pack_name)
                };
                let mut snippet = npc.public_summary.clone();
                if is_gm && let Some(agenda) = &npc.immediate_agenda {
                    snippet.push_str(&format!("\nAgenda: {agenda}"));
                }
                if is_gm && let Some(dossier) = &npc.gm_secret_dossier {
                    snippet.push_str(&format!("\n{dossier}"));
                }
                hits.push(GmLookupHit {
                    source_kind: GmLookupSourceKind::SessionNpc,
                    item_id: npc.character_id.clone(),
                    title: npc.character_name.clone(),
                    subtitle: format!(
                        "{kind_prefix} · {} · {}",
                        npc.callsign_or_role, npc.apparent_condition
                    ),
                    snippet: snippet.chars().take(420).collect(),
                    is_persistent: if is_db { npc.is_persistent } else { false },
                    score,
                    section_anchor: None,
                    can_delete: is_db && access.can_manage,
                    is_campaign_default: !is_db,
                });
            }
        }
    }

    let include_world_clocks = matches!(
        norm_scope.as_str(),
        "ALL" | "LORE" | "CAMPAIGN" | "CLOCKS" | "FACTIONS" | "WORLD" | "HEAT"
    );
    if include_world_clocks {
        let raw_ledger = crate::world_clocks::get_or_init_world_ledger(state, session_id).await;
        let ledger = satya_campaign::filter_world_ledger_for_viewer(&raw_ledger, is_gm);
        for clock in &ledger.faction_clocks {
            let mut body = format!(
                "{} {}/{} standing {:+} {}",
                clock.faction_id,
                clock.progress,
                clock.segments,
                clock.party_standing,
                clock.public_rumor
            );
            if is_gm && let Some(agenda) = &clock.gm_secret_agenda {
                body.push_str(&format!(" Agenda: {agenda}"));
            }
            let title = format!("{} — {}", clock.faction_name, clock.clock_title);
            let mut score = score_text_match(&q_low, &keywords, &title, &body);
            if matches!(norm_scope.as_str(), "CLOCKS" | "FACTIONS" | "WORLD")
                && score == 0
                && q_low.is_empty()
            {
                score = 5;
            }
            if score > 0 {
                let mut snippet = format!(
                    "Progress: {}/{} · Party Standing: {:+}\nRumor: {}",
                    clock.progress, clock.segments, clock.party_standing, clock.public_rumor
                );
                if is_gm && let Some(agenda) = &clock.gm_secret_agenda {
                    snippet.push_str(&format!("\nGM Secret Agenda: {agenda}"));
                }
                hits.push(GmLookupHit {
                    source_kind: GmLookupSourceKind::CampaignLore,
                    item_id: format!("faction-clock:{}", clock.faction_id),
                    title,
                    subtitle: format!(
                        "Faction Clock ({}/{}) · Standing {:+}",
                        clock.progress, clock.segments, clock.party_standing
                    ),
                    snippet: snippet.chars().take(420).collect(),
                    is_persistent: true,
                    score,
                    section_anchor: None,
                    can_delete: false,
                    is_campaign_default: true,
                });
            }
        }

        let heat_title = format!(
            "Operational Heat {}/10 ({})",
            ledger.operational_heat, ledger.heat_tier_code
        );
        let heat_body = format!(
            "operational heat jurisdiction {} {} {} black market police delta_ext {}",
            ledger.active_economic_tier.as_str(),
            ledger.active_zoning.as_str(),
            ledger.jurisdiction_response_profile,
            ledger.heat_description
        );
        let mut heat_score = score_text_match(&q_low, &keywords, &heat_title, &heat_body);
        if matches!(norm_scope.as_str(), "HEAT" | "WORLD" | "CLOCKS")
            && heat_score == 0
            && q_low.is_empty()
        {
            heat_score = 5;
        }
        if heat_score > 0 {
            hits.push(GmLookupHit {
                source_kind: GmLookupSourceKind::CampaignLore,
                item_id: "world-operational-heat".to_string(),
                title: heat_title,
                subtitle: "World Ledger · Jurisdiction & Heat".to_string(),
                snippet: format!(
                    "Tier: {} ({}) · Public Delta_ext: {:+} · Accounts Frozen: {} · Black Market Delta_ext: {:+}\nResponse: {}",
                    ledger.active_economic_tier.as_str(),
                    ledger.active_zoning.as_str(),
                    ledger.heat_public_check_delta_ext,
                    ledger.legal_bank_accounts_frozen,
                    ledger.black_market_delta_ext_mod,
                    ledger.jurisdiction_response_profile
                )
                .chars()
                .take(420)
                .collect(),
                is_persistent: true,
                score: heat_score,
                section_anchor: None,
                can_delete: false,
                is_campaign_default: true,
            });
        }
    }

    hits.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.title.cmp(&b.title)));
    hits.truncate(25);
}

/// Query parameters for `GET /api/campaign/lore/chapter` and `GET /api/sessions/:session_id/lore/chapter`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct LoreChapterQuery {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub spoiler_shield: bool,
}

fn classify_lore_file_visibility(rel_label: &str) -> &'static str {
    let rel_low = rel_label.to_ascii_lowercase().replace('\\', "/");
    if rel_low.contains("04-chronicle-and-gm/") || rel_low.ends_with("/prologue.md") {
        "GM_CONSPIRACY_SECRET"
    } else if rel_low.ends_with("satyang_and_four_pillars.md") {
        "ARC2_SATYANG"
    } else {
        "PUBLIC"
    }
}

fn parse_summary_order(root: &Path, lore_dir: &str) -> Vec<(String, String)> {
    let summary_path = root.join(lore_dir).join("SUMMARY.md");
    let Ok(raw) = std::fs::read_to_string(&summary_path) else {
        return Vec::new();
    };
    let clean_dir = lore_dir.trim_end_matches('/').replace('\\', "/");
    let mut part = String::from("Campaign Compendium");
    let mut ordered = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed.strip_prefix("### ") {
            let clean_part = heading
                .split("(`")
                .next()
                .unwrap_or(heading)
                .trim()
                .to_string();
            if !clean_part.is_empty() {
                part = clean_part;
            }
            continue;
        }
        if let Some(idx) = trimmed.find("](")
            && let Some(end_rel) = trimmed[idx + 2..].find(')')
        {
            let link = trimmed[idx + 2..idx + 2 + end_rel].trim();
            if link.ends_with(".md")
                && !link.contains("..")
                && !link.starts_with('/')
                && !link.eq_ignore_ascii_case("SUMMARY.md")
            {
                let full_rel = if clean_dir.is_empty() {
                    link.to_string()
                } else {
                    format!("{clean_dir}/{}", link.trim_start_matches("./"))
                };
                ordered.push((full_rel, part.clone()));
            }
        }
    }
    ordered
}

fn extract_chapter_title(content: &str, rel_label: &str) -> String {
    content
        .lines()
        .find_map(|l| l.trim().strip_prefix("# ").map(|t| t.trim().to_string()))
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| {
            Path::new(rel_label)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(rel_label)
                .replace('_', " ")
        })
}

fn collect_all_pack_lore_entries(
    pack: &ResolvedPackDefaults,
    root: &Path,
) -> Vec<(CampaignLoreChapterSummary, PathBuf)> {
    let mut seen_paths = BTreeSet::new();
    let mut ordered_candidates: Vec<(String, String, PathBuf)> = Vec::new();

    for rel_dir in &pack.lore_dirs {
        for (rel_label, part) in parse_summary_order(root, rel_dir) {
            let abs = root.join(&rel_label);
            if abs.is_file() && seen_paths.insert(rel_label.clone()) {
                ordered_candidates.push((rel_label, part, abs));
            }
        }
        let mut md_files = Vec::new();
        collect_markdown_files(&root.join(rel_dir), &mut md_files);
        for abs in md_files {
            let rel_label = abs
                .strip_prefix(root)
                .unwrap_or(&abs)
                .to_string_lossy()
                .replace('\\', "/");
            if seen_paths.insert(rel_label.clone()) {
                let fallback_part = Path::new(&rel_label)
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|s| s.to_str())
                    .unwrap_or("Campaign Lore")
                    .to_string();
                ordered_candidates.push((rel_label, fallback_part, abs));
            }
        }
    }

    let mut result = Vec::new();
    for (rel_label, part, abs) in ordered_candidates {
        let Ok(content) = std::fs::read_to_string(&abs) else {
            continue;
        };
        let headings = extract_markdown_headings(&content)
            .into_iter()
            .filter(|h| !h.is_empty())
            .collect();
        let summary = CampaignLoreChapterSummary {
            title: extract_chapter_title(&content, &rel_label),
            part,
            visibility: classify_lore_file_visibility(&rel_label).to_string(),
            relative_path: rel_label,
            headings,
        };
        result.push((summary, abs));
    }
    result
}

fn filter_markdown_for_viewer(rel_label: &str, raw: &str, can_see_secrets: bool) -> String {
    if can_see_secrets {
        return raw.to_string();
    }
    let mut kept = Vec::new();
    for (idx, section) in raw.split("\n## ").enumerate() {
        let block = if idx == 0 {
            section.to_string()
        } else {
            format!("## {section}")
        };
        if idx == 0 || is_public_campaign_lore(rel_label, &block) {
            kept.push(block);
        }
    }
    kept.join("\n\n")
}

fn lore_err(status: StatusCode, msg: impl std::fmt::Display) -> Box<axum::response::Response> {
    Box::new((status, Json(json!({ "error": msg.to_string() }))).into_response())
}

async fn authorize_session_lore_viewer(
    state: &SignalingAppState,
    headers: &HeaderMap,
    sid: &str,
    spoiler_shield: bool,
) -> Result<bool, Box<axum::response::Response>> {
    if sid.is_empty() {
        return Err(lore_err(
            StatusCode::BAD_REQUEST,
            "session_id cannot be empty",
        ));
    }
    let (user_opt, is_gm_service) = if verify_gm_service_header(state, headers).is_ok() {
        (None, true)
    } else {
        let Some(auth_hdr) = headers.get("Authorization").and_then(|h| h.to_str().ok()) else {
            return Err(lore_err(
                StatusCode::UNAUTHORIZED,
                "Missing Authorization header",
            ));
        };
        let Some(user) = state.auth.verify_auth_token(auth_hdr).await else {
            return Err(lore_err(
                StatusCode::UNAUTHORIZED,
                "Invalid or expired auth token",
            ));
        };
        if !state.auth.sessions.read().await.contains_key(sid) {
            let _ = state.auth.rehydrate_session_from_db(sid).await;
        }
        if let Err(err) = state.auth.resolve_authorized_member_role(sid, &user).await {
            return Err(lore_err(StatusCode::FORBIDDEN, err));
        }
        (Some(user), false)
    };
    Ok(can_caller_view_gm_secrets_with_shield(
        state,
        sid,
        user_opt.as_ref(),
        is_gm_service,
        spoiler_shield,
    )
    .await)
}

fn build_lore_catalog_response(
    pack: &ResolvedPackDefaults,
    root: &Path,
    can_see_secrets: bool,
) -> CampaignLoreCatalogResponse {
    let chapters = collect_all_pack_lore_entries(pack, root)
        .into_iter()
        .map(|(s, _)| s)
        .filter(|s| can_see_secrets || s.visibility == "PUBLIC")
        .collect();
    CampaignLoreCatalogResponse {
        pack_name: pack.pack_name.clone(),
        chapters,
    }
}

fn read_allowed_lore_chapter(
    pack: &ResolvedPackDefaults,
    root: &Path,
    requested_path: &str,
    can_see_secrets: bool,
) -> Result<CampaignLoreChapterDocument, Box<axum::response::Response>> {
    let norm_req = requested_path.trim().replace('\\', "/");
    if norm_req.is_empty() {
        return Err(lore_err(StatusCode::BAD_REQUEST, "path query is required"));
    }
    let entries = collect_all_pack_lore_entries(pack, root);
    let Some((mut summary, abs_path)) = entries
        .into_iter()
        .find(|(s, _)| s.relative_path == norm_req)
    else {
        return Err(lore_err(
            StatusCode::NOT_FOUND,
            format!("Campaign lore chapter '{norm_req}' was not found"),
        ));
    };
    if !can_see_secrets && summary.visibility != "PUBLIC" {
        return Err(lore_err(
            StatusCode::FORBIDDEN,
            "Campaign lore chapter requires GM/Co-GM clearance without Spoiler Shield",
        ));
    }
    let raw = std::fs::read_to_string(&abs_path).map_err(|e| {
        lore_err(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to read campaign lore chapter: {e}"),
        )
    })?;
    let markdown = filter_markdown_for_viewer(&summary.relative_path, &raw, can_see_secrets);
    summary.headings = extract_markdown_headings(&markdown);
    let rendered_html = render_rules_markdown_to_html(&markdown);
    Ok(CampaignLoreChapterDocument {
        pack_name: pack.pack_name.clone(),
        summary,
        markdown,
        rendered_html,
    })
}

/// Lists `PUBLIC` campaign lore chapters (`GET /api/campaign/lore`).
pub async fn list_public_lore_handler(State(state): State<Arc<SignalingAppState>>) -> impl IntoResponse {
    let pack = resolve_pack_defaults(Some(state.campaign_pack.as_str()));
    let root = workspace_root();
    (
        StatusCode::OK,
        Json(build_lore_catalog_response(&pack, &root, false)),
    )
        .into_response()
}

/// Reads an allow-listed `PUBLIC` campaign lore chapter (`GET /api/campaign/lore/chapter?path=...`).
pub async fn get_public_lore_chapter_handler(
    State(state): State<Arc<SignalingAppState>>,
    Query(query): Query<LoreChapterQuery>,
) -> impl IntoResponse {
    let pack = resolve_pack_defaults(Some(state.campaign_pack.as_str()));
    let root = workspace_root();
    match read_allowed_lore_chapter(&pack, &root, &query.path, false) {
        Ok(doc) => (StatusCode::OK, Json(doc)).into_response(),
        Err(resp) => *resp,
    }
}

/// Lists role- and Spoiler-Shield-filtered campaign lore chapters for a session (`GET /api/sessions/:session_id/lore`).
pub async fn list_session_lore_handler(
    State(state): State<Arc<SignalingAppState>>,
    AxumPath(session_id): AxumPath<String>,
    Query(shield): Query<SpoilerShieldQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let can_see_secrets = match authorize_session_lore_viewer(
        &state,
        &headers,
        session_id.trim(),
        shield.spoiler_shield,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return *resp,
    };
    let pack = resolve_pack_defaults(Some(state.campaign_pack.as_str()));
    let root = workspace_root();
    (
        StatusCode::OK,
        Json(build_lore_catalog_response(&pack, &root, can_see_secrets)),
    )
        .into_response()
}

/// Reads a role- and Spoiler-Shield-filtered campaign lore chapter for a session (`GET /api/sessions/:session_id/lore/chapter?path=...`).
pub async fn get_session_lore_chapter_handler(
    State(state): State<Arc<SignalingAppState>>,
    AxumPath(session_id): AxumPath<String>,
    Query(query): Query<LoreChapterQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let can_see_secrets = match authorize_session_lore_viewer(
        &state,
        &headers,
        session_id.trim(),
        query.spoiler_shield,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return *resp,
    };
    let pack = resolve_pack_defaults(Some(state.campaign_pack.as_str()));
    let root = workspace_root();
    match read_allowed_lore_chapter(&pack, &root, &query.path, can_see_secrets) {
        Ok(doc) => (StatusCode::OK, Json(doc)).into_response(),
        Err(resp) => *resp,
    }
}
