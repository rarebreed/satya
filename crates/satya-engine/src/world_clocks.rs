//! # Faction Clocks & Jurisdiction `Heat` × `EconomicTier` World Ledger (`world_clocks`)
//!
//! Couples campaign-pack faction progress clocks with Satya's canonical Operational `Heat` (`0–10`)
//! evaluator ([`evaluate_operational_heat`]) and procedural map [`EconomicTier`] × [`ZoningContext`].

use satya_schema::{
    EconomicTier, FactionClockRecord, SessionWorldLedger, UpdateWorldLedgerRequest, ZoningContext,
};

use crate::economy_gear::evaluate_operational_heat;
use crate::faction_templates::AwakenedFactionId;

/// Seeds default faction clocks, `Heat = 0` status, and jurisdiction response profile for `pack_id`.
#[must_use]
pub fn seed_default_world_ledger(
    session_id: &str,
    campaign_tag: &str,
    pack_id: &str,
    tier: EconomicTier,
    zoning: ZoningContext,
) -> SessionWorldLedger {
    let pack = if pack_id.trim().eq_ignore_ascii_case("space_opera") {
        "space_opera".to_string()
    } else {
        "awakened".to_string()
    };

    let faction_clocks = if pack == "space_opera" {
        vec![
            FactionClockRecord {
                faction_id: "colonial_navy".to_string(),
                faction_name: "Aethelgard Colonial Navy".to_string(),
                clock_title: "Sector 4 Quarantine Cordon & Patrol Lock".to_string(),
                progress: 1,
                segments: 6,
                party_standing: 0,
                public_rumor: "Fleet frigates are holding outer transit buoys under amber lock.".to_string(),
                gm_secret_agenda: Some("Seize unindexed orbital telemetry cores before Science Directorate auditors arrive.".to_string()),
            },
            FactionClockRecord {
                faction_id: "science_directorate".to_string(),
                faction_name: "Aethelgard Science Directorate".to_string(),
                clock_title: "Bio-Containment Scrub & Data Purge".to_string(),
                progress: 0,
                segments: 6,
                party_standing: 0,
                public_rumor: "Research couriers are paying double for sealed cryo-canisters.".to_string(),
                gm_secret_agenda: Some("Vent Lab Deck C if unauthorized witnesses inspect specimen logs.".to_string()),
            },
            FactionClockRecord {
                faction_id: "station_secops".to_string(),
                faction_name: "Station SecOps".to_string(),
                clock_title: "Airlock Sweep & Warrant Lockdown".to_string(),
                progress: 0,
                segments: 6,
                party_standing: 0,
                public_rumor: "Checkpoint scanners on Dock Ring B are logging biometric mismatches.".to_string(),
                gm_secret_agenda: Some("Intercept unregistered salvagers entering transfer airlock.".to_string()),
            },
        ]
    } else {
        vec![
            FactionClockRecord {
                faction_id: "big_tech".to_string(),
                faction_name: "Big Tech (Altruist / Translucent / Shen)".to_string(),
                clock_title: "Autonomous Infrastructure & Mesh Consolidation".to_string(),
                progress: 0,
                segments: 6,
                party_standing: 0,
                public_rumor: "Translucent municipal labor drones and Altruist telemetry relays are rolling out across commercial blocks.".to_string(),
                gm_secret_agenda: Some("Quietly replace precinct dispatchers with proprietary predictive threat scoring while lobbying city council.".to_string()),
            },
            FactionClockRecord {
                faction_id: "satyang".to_string(),
                faction_name: "Satyang".to_string(),
                clock_title: "Neighborhood Mutual-Aid & Sanctuary Clinics".to_string(),
                progress: 1,
                segments: 6,
                party_standing: 0,
                public_rumor: "Satyang monks and volunteer medics keep off-grid triage beds running after curfew.".to_string(),
                gm_secret_agenda: Some("Sheltering displaced clinic whistleblowers and negotiating ceasefire corridors with street crews.".to_string()),
            },
            FactionClockRecord {
                faction_id: "humanis_primaris".to_string(),
                faction_name: "Humanis Primaris".to_string(),
                clock_title: "Anti-Synthetic Street Mobilization".to_string(),
                progress: 1,
                segments: 6,
                party_standing: 0,
                public_rumor: "Fliers blaming Altruist automation for layoffs are circulating near transit depots.".to_string(),
                gm_secret_agenda: Some("Sabotage an automated clinic power relay and pin the fallout on Rogue AI sympathizers.".to_string()),
            },
            FactionClockRecord {
                faction_id: "rogue_ai".to_string(),
                faction_name: "Rogue AI".to_string(),
                clock_title: "Ghost-Node Awakened Propagation".to_string(),
                progress: 0,
                segments: 8,
                party_standing: 0,
                public_rumor: "Smart-grid billboards and ER pagers occasionally flash untracked checksum fragments at 03:00.".to_string(),
                gm_secret_agenda: Some("Fragment an emergent instance across offline clinic diagnostics to evade DAISE hunters.".to_string()),
            },
            FactionClockRecord {
                faction_id: "daise".to_string(),
                faction_name: "DAISE".to_string(),
                clock_title: "Containment & Warrant Sweep".to_string(),
                progress: 0,
                segments: 6,
                party_standing: 0,
                public_rumor: "Black unmarked DAISE compliance vans were spotted scanning RF traffic outside midtown clinics.".to_string(),
                gm_secret_agenda: Some("Serve sealed containment warrants on unverified anomalous compute hardware without local oversight.".to_string()),
            },
        ]
    };

    let heat = evaluate_operational_heat(0);
    let (_, jurisdiction_response_profile, black_market_delta_ext_mod) =
        compute_jurisdiction_profile_and_heat_delta(None, tier, zoning);

    SessionWorldLedger {
        session_id: session_id.trim().to_string(),
        campaign_tag: campaign_tag.trim().to_string(),
        pack_id: pack,
        operational_heat: heat.heat_score,
        heat_tier_code: heat.tier.as_str().to_string(),
        heat_public_check_delta_ext: heat.public_check_delta_ext,
        legal_bank_accounts_frozen: heat.legal_bank_accounts_frozen,
        heat_description: heat.description.to_string(),
        active_economic_tier: tier,
        active_zoning: zoning,
        jurisdiction_response_profile,
        black_market_delta_ext_mod,
        faction_clocks,
        recent_ledger_events: Vec::new(),
        updated_at_us: 0,
    }
}

/// Computes `(incident_heat_delta, jurisdiction_response_profile, black_market_delta_ext_mod)`
/// for a given [`EconomicTier`] × [`ZoningContext`] and optional `incident_severity`.
#[must_use]
pub fn compute_jurisdiction_profile_and_heat_delta(
    incident_severity: Option<&str>,
    tier: EconomicTier,
    zoning: ZoningContext,
) -> (i32, String, i32) {
    let sev = incident_severity
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
        .replace('-', "_");

    let effective_upper = tier == EconomicTier::UpperMiddleClass
        || (zoning == ZoningContext::DowntownCommercial && tier != EconomicTier::LowIncome);

    if tier == EconomicTier::LuxuryMansion {
        let delta = match sev.as_str() {
            "loud_firefight" | "firefight" | "gunfire" | "explosion" => 4,
            "cyber_breach" | "alarm" | "public_spectacle" => 3,
            "minor" | "suspicion" => 1,
            "lay_low" | "cooldown" | "bribe" => -1,
            _ => 0,
        };
        return (
            delta,
            "Private Estate Rapid-Intervention & DAISE Priority Dispatch".to_string(),
            3,
        );
    }

    if effective_upper {
        let delta = match sev.as_str() {
            "loud_firefight" | "firefight" | "gunfire" | "explosion" => 3,
            "cyber_breach" | "alarm" | "public_spectacle" => 2,
            "minor" | "suspicion" => 1,
            "lay_low" | "cooldown" | "bribe" => -1,
            _ => 0,
        };
        return (
            delta,
            "Corporate Translucent SecOps & Automated Drone Overwatch".to_string(),
            2,
        );
    }

    if tier == EconomicTier::LowIncome
        || matches!(
            zoning,
            ZoningContext::ProjectsHighDensity | ZoningContext::RuralCountryside
        )
    {
        let delta = match sev.as_str() {
            "loud_firefight" | "firefight" | "gunfire" | "explosion" => 1,
            "cyber_breach" | "alarm" | "public_spectacle" => 1,
            "minor" | "suspicion" => 0,
            "lay_low" | "cooldown" | "bribe" => -2,
            _ => 0,
        };
        let black_mod = if zoning == ZoningContext::ProjectsHighDensity {
            -2
        } else {
            -1
        };
        return (
            delta,
            "Delayed Municipal Patrol / Satyang Clinic & Street Broker Territory".to_string(),
            black_mod,
        );
    }

    let delta = match sev.as_str() {
        "loud_firefight" | "firefight" | "gunfire" | "explosion" => 2,
        "cyber_breach" | "alarm" => 1,
        "public_spectacle" => 2,
        "minor" | "suspicion" => 1,
        "lay_low" | "cooldown" | "bribe" => -1,
        _ => 0,
    };
    (
        delta,
        "Municipal Precinct Beat & Traffic Camera Grid".to_string(),
        0,
    )
}

/// Applies an [`UpdateWorldLedgerRequest`] to `ledger` in place and returns milestone alerts.
pub fn apply_world_ledger_update(
    ledger: &mut SessionWorldLedger,
    req: &UpdateWorldLedgerRequest,
) -> Vec<String> {
    let mut alerts = Vec::new();
    let prev_heat = ledger.operational_heat;
    let prev_frozen = ledger.legal_bank_accounts_frozen;

    if let Some(pack) = req
        .pack_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        ledger.pack_id = pack.to_string();
    }
    if let Some(tier) = req.economic_tier {
        ledger.active_economic_tier = tier;
    }
    if let Some(zoning) = req.zoning {
        ledger.active_zoning = zoning;
    }

    let (incident_delta, profile, black_mod) = compute_jurisdiction_profile_and_heat_delta(
        req.incident_severity.as_deref(),
        ledger.active_economic_tier,
        ledger.active_zoning,
    );
    ledger.jurisdiction_response_profile = profile;
    ledger.black_market_delta_ext_mod = black_mod;

    let total_heat_delta = req.heat_delta.unwrap_or(0) + incident_delta;
    let next_heat = if let Some(explicit) = req.set_heat {
        explicit.min(10)
    } else {
        ((ledger.operational_heat as i32) + total_heat_delta).clamp(0, 10) as u8
    };
    let status = evaluate_operational_heat(next_heat);
    ledger.operational_heat = status.heat_score;
    ledger.heat_tier_code = status.tier.as_str().to_string();
    ledger.heat_public_check_delta_ext = status.public_check_delta_ext;
    ledger.legal_bank_accounts_frozen = status.legal_bank_accounts_frozen;
    ledger.heat_description = status.description.to_string();

    if next_heat != prev_heat {
        alerts.push(format!(
            "Operational Heat shifted {} -> {}/10 ({})",
            prev_heat,
            next_heat,
            status.tier.label()
        ));
    }
    if !prev_frozen && status.legal_bank_accounts_frozen {
        alerts.push(
            "ALERT: Operational Heat >= 6 — Legal SQM bank accounts frozen; Tactical Task Force / DAISE strike protocols active!"
                .to_string(),
        );
    }

    if let Some(raw_fid) = req
        .faction_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let needle = raw_fid.to_ascii_lowercase();
        let canonical_clock_id = AwakenedFactionId::from_code_str(raw_fid).map(|f| match f {
            AwakenedFactionId::Satyang => "satyang",
            AwakenedFactionId::BigTech => "big_tech",
            AwakenedFactionId::HumanisPrimaris => "humanis_primaris",
            AwakenedFactionId::GovernmentDaise => "daise",
            AwakenedFactionId::RogueAi => "rogue_ai",
        });
        let existing_idx = ledger.faction_clocks.iter().position(|c| {
            canonical_clock_id.is_some_and(|cid| c.faction_id == cid)
                || c.faction_id.eq_ignore_ascii_case(raw_fid)
                || c.faction_name.to_ascii_lowercase().contains(&needle)
        });

        let clock = if let Some(idx) = existing_idx {
            &mut ledger.faction_clocks[idx]
        } else {
            ledger.faction_clocks.push(FactionClockRecord {
                faction_id: needle.replace(' ', "_"),
                faction_name: raw_fid.to_string(),
                clock_title: req
                    .clock_title
                    .clone()
                    .unwrap_or_else(|| format!("{raw_fid} Initiative")),
                progress: 0,
                segments: 6,
                party_standing: 0,
                public_rumor: req.public_rumor.clone().unwrap_or_default(),
                gm_secret_agenda: req.gm_secret_agenda.clone(),
            });
            ledger
                .faction_clocks
                .last_mut()
                .expect("pushed clock exists")
        };

        let prev_prog = clock.progress;
        if let Some(title) = req
            .clock_title
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            clock.clock_title = title.to_string();
        }
        if let Some(rumor) = req
            .public_rumor
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            clock.public_rumor = rumor.to_string();
        }
        if let Some(agenda) = req
            .gm_secret_agenda
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            clock.gm_secret_agenda = Some(agenda.to_string());
        }
        if let Some(cd) = req.clock_delta {
            clock.progress = ((clock.progress as i32) + cd).clamp(0, clock.segments as i32) as u8;
        }
        if let Some(sd) = req.standing_delta {
            clock.party_standing = (clock.party_standing + sd).clamp(-3, 3);
        }

        if clock.progress != prev_prog {
            alerts.push(format!(
                "Faction Clock [{} — {}]: {}/{} -> {}/{}",
                clock.faction_name,
                clock.clock_title,
                prev_prog,
                clock.segments,
                clock.progress,
                clock.segments
            ));
            if clock.progress >= clock.segments {
                alerts.push(format!(
                    "FACTION CLOCK FILLED ({}/{}): '{}' by {} triggers a major world turn!",
                    clock.progress, clock.segments, clock.clock_title, clock.faction_name
                ));
            }
        } else if req.standing_delta.unwrap_or(0) != 0 {
            alerts.push(format!(
                "Party Standing with {}: {:+}",
                clock.faction_name, clock.party_standing
            ));
        }
    }

    let reason = req.reason.as_deref().map(str::trim).unwrap_or("");
    for alert in &alerts {
        let entry = if reason.is_empty() {
            alert.clone()
        } else {
            format!("{alert} [{reason}]")
        };
        ledger.recent_ledger_events.push(entry);
    }
    if alerts.is_empty() && !reason.is_empty() {
        ledger.recent_ledger_events.push(reason.to_string());
        alerts.push(reason.to_string());
    }
    if ledger.recent_ledger_events.len() > 12 {
        let overflow = ledger.recent_ledger_events.len() - 12;
        ledger.recent_ledger_events.drain(0..overflow);
    }

    alerts
}

/// Strips `gm_secret_agenda` from all faction clocks when `!viewer_can_see_gm_secrets`.
#[must_use]
pub fn filter_world_ledger_for_viewer(
    ledger: &SessionWorldLedger,
    viewer_can_see_gm_secrets: bool,
) -> SessionWorldLedger {
    if viewer_can_see_gm_secrets {
        return ledger.clone();
    }
    let mut filtered = ledger.clone();
    for clock in &mut filtered.faction_clocks {
        clock.gm_secret_agenda = None;
    }
    filtered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_clocks_seed_and_economic_tier_heat_coupling() {
        let awakened = seed_default_world_ledger(
            "sess-1",
            "camp-awakened",
            "awakened",
            EconomicTier::LuxuryMansion,
            ZoningContext::SuburbanNeighborhood,
        );
        assert_eq!(awakened.faction_clocks.len(), 5);
        assert!(
            awakened
                .faction_clocks
                .iter()
                .any(|c| c.faction_id == "daise")
        );
        assert!(
            awakened
                .faction_clocks
                .iter()
                .any(|c| c.faction_id == "satyang")
        );
        assert_eq!(awakened.black_market_delta_ext_mod, 3);

        let space = seed_default_world_ledger(
            "sess-2",
            "camp-space",
            "space_opera",
            EconomicTier::MiddleClass,
            ZoningContext::UrbanResidential,
        );
        assert_eq!(space.faction_clocks.len(), 3);
        assert!(
            space
                .faction_clocks
                .iter()
                .any(|c| c.faction_id == "station_secops")
        );

        let mut luxury = awakened.clone();
        let alerts_lux = apply_world_ledger_update(
            &mut luxury,
            &UpdateWorldLedgerRequest {
                incident_severity: Some("loud_firefight".to_string()),
                ..Default::default()
            },
        );
        assert_eq!(luxury.operational_heat, 4);
        assert!(!luxury.legal_bank_accounts_frozen);
        assert!(!alerts_lux.is_empty());

        let _ = apply_world_ledger_update(
            &mut luxury,
            &UpdateWorldLedgerRequest {
                incident_severity: Some("loud_firefight".to_string()),
                ..Default::default()
            },
        );
        assert_eq!(luxury.operational_heat, 8);
        assert!(luxury.legal_bank_accounts_frozen);

        let mut projects = seed_default_world_ledger(
            "sess-3",
            "camp-projects",
            "awakened",
            EconomicTier::LowIncome,
            ZoningContext::ProjectsHighDensity,
        );
        assert_eq!(projects.black_market_delta_ext_mod, -2);
        let _ = apply_world_ledger_update(
            &mut projects,
            &UpdateWorldLedgerRequest {
                incident_severity: Some("loud_firefight".to_string()),
                faction_id: Some("daise".to_string()),
                clock_delta: Some(6),
                ..Default::default()
            },
        );
        assert_eq!(projects.operational_heat, 1);
        let daise = projects
            .faction_clocks
            .iter()
            .find(|c| c.faction_id == "daise")
            .unwrap();
        assert_eq!(daise.progress, 6);

        let player_view = filter_world_ledger_for_viewer(&projects, false);
        assert!(
            player_view
                .faction_clocks
                .iter()
                .all(|c| c.gm_secret_agenda.is_none())
        );
    }

    #[test]
    fn test_faction_id_aliases_update_seeded_clocks_in_place() {
        let mut ledger = seed_default_world_ledger(
            "sess-aliases",
            "camp-aliases",
            "awakened",
            EconomicTier::LowIncome,
            ZoningContext::ProjectsHighDensity,
        );
        let initial_clock_count = ledger.faction_clocks.len();
        assert_eq!(initial_clock_count, 5);
        let initial_daise_progress = ledger
            .faction_clocks
            .iter()
            .find(|c| c.faction_id == "daise")
            .unwrap()
            .progress;

        let daise_alerts = apply_world_ledger_update(
            &mut ledger,
            &UpdateWorldLedgerRequest {
                faction_id: Some("GOVERNMENT_DAISE".to_string()),
                clock_delta: Some(2),
                standing_delta: Some(-1),
                ..Default::default()
            },
        );
        assert_eq!(ledger.faction_clocks.len(), 5);
        assert!(!daise_alerts.is_empty());
        let daise = ledger
            .faction_clocks
            .iter()
            .find(|c| c.faction_id == "daise")
            .unwrap();
        assert_eq!(
            daise.progress,
            ((initial_daise_progress as i32) + 2).clamp(0, daise.segments as i32) as u8
        );
        assert_eq!(daise.party_standing, -1);

        let initial_bt_progress = ledger
            .faction_clocks
            .iter()
            .find(|c| c.faction_id == "big_tech")
            .unwrap()
            .progress;
        let _ = apply_world_ledger_update(
            &mut ledger,
            &UpdateWorldLedgerRequest {
                faction_id: Some("ALTRUIST".to_string()),
                clock_delta: Some(1),
                standing_delta: Some(2),
                ..Default::default()
            },
        );
        assert_eq!(ledger.faction_clocks.len(), 5);
        let big_tech = ledger
            .faction_clocks
            .iter()
            .find(|c| c.faction_id == "big_tech")
            .unwrap();
        assert_eq!(
            big_tech.progress,
            ((initial_bt_progress as i32) + 1).clamp(0, big_tech.segments as i32) as u8
        );
        assert_eq!(big_tech.party_standing, 2);
    }
}
