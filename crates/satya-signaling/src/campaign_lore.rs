//! Active campaign pack resolution (`awakened` default vs `space_opera` optional),
//! TOML/Markdown campaign lore omnisearch, and starter NPC profile loading for `satya-signaling`.

use crate::npcs::load_session_npcs_for_omnisearch;
use crate::state::SignalingAppState;
use satya_maps::SectorPreset;
use satya_schema::{GmLookupHit, GmLookupSourceKind, SessionNpcEntry};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

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

/// Resolves the active campaign pack name via:
/// `SATYA_CAMPAIGN_PACK` env > non-default `explicit_pack` (`state.campaign_pack`) >
/// `config/ai.toml` `[ai_gm].campaign_pack` > `config/campaign.toml` `active_pack` > `"awakened"`.
pub fn resolve_active_pack_name(explicit_pack: Option<&str>) -> String {
    if let Ok(env_val) = std::env::var("SATYA_CAMPAIGN_PACK") {
        let trimmed = env_val.trim().to_ascii_lowercase();
        if !trimmed.is_empty() {
            return trimmed;
        }
    }
    if let Some(exp) = explicit_pack {
        let trimmed = exp.trim().to_ascii_lowercase();
        if !trimmed.is_empty() && trimmed != "awakened" {
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
    if let Some(camp_tbl) = read_toml_table(&root.join("config/campaign.toml"))
        && let Some(active) = camp_tbl.get("active_pack").and_then(|v| v.as_str())
    {
        let trimmed = active.trim().to_ascii_lowercase();
        if !trimmed.is_empty() {
            return trimmed;
        }
    }
    if let Some(exp) = explicit_pack {
        let trimmed = exp.trim().to_ascii_lowercase();
        if !trimmed.is_empty() {
            return trimmed;
        }
    }
    "awakened".to_string()
}

/// Resolves full pack defaults (sector preset fallback, token labels, lore dirs, NPC TOML).
pub fn resolve_pack_defaults(explicit_pack: Option<&str>) -> ResolvedPackDefaults {
    let pack_name = resolve_active_pack_name(explicit_pack);
    let mut defaults = if pack_name == "space_opera" {
        ResolvedPackDefaults {
            pack_name: pack_name.clone(),
            default_sector_preset: SectorPreset::OrbitalAirlock,
            hostile_token_label: "SecOps Sentry".to_string(),
            ally_token_label: "Capt. Vance".to_string(),
            default_ai_player_name: "Kaelen Voss (AI Player)".to_string(),
            lore_dirs: Vec::new(),
            npc_toml_file: "data/npcs/space_opera_npcs.toml".to_string(),
        }
    } else {
        ResolvedPackDefaults {
            pack_name: pack_name.clone(),
            default_sector_preset: SectorPreset::UrbanCrossroads,
            hostile_token_label: "DAISE Tactical Agent".to_string(),
            ally_token_label: "Dr. Bandajara".to_string(),
            default_ai_player_name: "Mateo Vance (AI Player)".to_string(),
            lore_dirs: vec!["docs/campaign".to_string()],
            npc_toml_file: "data/npcs/awakened_npcs.toml".to_string(),
        }
    };

    let root = workspace_root();
    if let Some(camp_tbl) = read_toml_table(&root.join("config/campaign.toml"))
        && let Some(pack_tbl) = camp_tbl
            .get("packs")
            .and_then(|v| v.as_table())
            .and_then(|t| t.get(&pack_name))
            .and_then(|v| v.as_table())
    {
        if let Some(code) = pack_tbl
            .get("default_sector_preset")
            .and_then(|v| v.as_str())
            && let Some(parsed) = SectorPreset::from_code_str(code)
        {
            defaults.default_sector_preset = parsed;
        }
        if let Some(lbl) = pack_tbl
            .get("default_hostile_token")
            .and_then(|v| v.as_table())
            .and_then(|t| t.get("label"))
            .and_then(|v| v.as_str())
        {
            defaults.hostile_token_label = lbl.to_string();
        }
        if let Some(lbl) = pack_tbl
            .get("default_ally_token")
            .and_then(|v| v.as_table())
            .and_then(|t| t.get("label"))
            .and_then(|v| v.as_str())
        {
            defaults.ally_token_label = lbl.to_string();
        }
        if let Some(dirs) = pack_tbl.get("lore_dirs").and_then(|v| v.as_array()) {
            defaults.lore_dirs = dirs
                .iter()
                .filter_map(|v| v.as_str().map(ToString::to_string))
                .collect();
        }
        if let Some(npc_file) = pack_tbl.get("npc_toml_file").and_then(|v| v.as_str()) {
            defaults.npc_toml_file = npc_file.to_string();
        }
    }

    defaults
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
    ]
    .iter()
    .any(|marker| body_low.contains(marker))
}

/// Appends ranked [`GmLookupSourceKind::CampaignLore`] (Markdown + TOML) and
/// [`GmLookupSourceKind::SessionNpc`] (SQLite `session_npcs` + pack starter TOML)
/// hits into `hits`, redacting GM secrets unless `is_gm` is true.
pub async fn append_omnisearch_lore_and_npc_hits(
    state: &SignalingAppState,
    session_id: &str,
    is_gm: bool,
    query: &str,
    scope: &str,
    hits: &mut Vec<GmLookupHit>,
) {
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
                let kind_prefix = if is_db { "Session NPC" } else { "Starter NPC" };
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
                    is_persistent: npc.is_persistent,
                    score,
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
            });
        }
    }

    hits.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.title.cmp(&b.title)));
    hits.truncate(25);
}
