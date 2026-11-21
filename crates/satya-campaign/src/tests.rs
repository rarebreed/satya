//! Unit tests for `satya-campaign` covering NPC generation, party motivational hook
//! compilation, and *Awakened* faction/archetype legality audits.

use std::collections::BTreeMap;

use satya_engine::character_audit::verify_character_sheet_legality;
use satya_schema::CharacterSheetRecord;

use crate::faction_templates::{AwakenedFactionId, FactionArchetypeId};
use crate::hook_weaver::{HookCategory, compile_party_hooks, derive_npc_motivational_profile};
use crate::npc_generator::{NpcGenerationParams, NpcTemplateKind, generate_npc_from_prompt};

#[test]
fn test_derive_motivational_profile_highlights_dual_psyche_wedge() {
    let npc_gen = generate_npc_from_prompt(&NpcGenerationParams {
        prompt: "Terry is a kind general MD, but needs to feel like she belongs".to_string(),
        name_override: None,
        template: NpcTemplateKind::Auto,
        faction_archetype: None,
        career_terms: 2,
        seed: 9101,
        owner_user_id: "gm".to_string(),
    });
    let profile = derive_npc_motivational_profile(
        &npc_gen.character.characteristics,
        npc_gen.inferred_template,
        "Terry is a kind general MD, but needs to feel like she belongs",
        &npc_gen.character.granted_access_tags,
        "awakened",
    );
    assert!(
        profile.negotiation_lever.contains("BLG=16") || profile.negotiation_lever.contains("SLF=5")
    );
    assert!(profile.dual_psyche_wedge.contains("Wedge"));
    assert!(!profile.dominant_poles.is_empty());
}

#[test]
fn test_compile_party_hooks_generates_character_and_entanglement_hooks() {
    let mut pc1 = CharacterSheetRecord {
        character_id: "pc-maya".to_string(),
        character_name: "Maya Chen".to_string(),
        callsign_or_role: "Field Diagnostician".to_string(),
        origin_label: "Metro Clinic".to_string(),
        operational_heat: 5,
        granted_access_tags: vec!["DAISE_MED_L2".to_string()],
        ..CharacterSheetRecord::default()
    };
    pc1.characteristics.opn = 17;
    pc1.characteristics.blg = 16;
    pc1.characteristics.cps = 16;
    pc1.skills
        .insert("clinical_diagnosis_internal_medicine".to_string(), 5);

    let mut pc2 = CharacterSheetRecord {
        character_id: "pc-reeves".to_string(),
        character_name: "Harlan Reeves".to_string(),
        callsign_or_role: "Humanis Liaison".to_string(),
        ..CharacterSheetRecord::default()
    };
    pc2.characteristics.opn = 5;
    pc2.characteristics.cnf = 16;
    pc2.characteristics.trs = 4;

    let mut ctx = BTreeMap::new();
    ctx.insert("location".to_string(), "St. Jude Triage Annex".to_string());
    ctx.insert("sector_preset".to_string(), "hospital".to_string());

    let manifest = compile_party_hooks(&[pc1, pc2], &ctx, "awakened");
    assert_eq!(manifest.pack_id, "awakened");
    assert!(
        manifest
            .hooks
            .iter()
            .any(|h| h.category == HookCategory::IdentityPsyche)
    );
    assert!(
        manifest
            .hooks
            .iter()
            .any(|h| h.category == HookCategory::SkillSpotlight)
    );
    assert!(
        manifest
            .hooks
            .iter()
            .any(|h| h.category == HookCategory::CredentialAccess)
    );
    assert!(
        manifest
            .hooks
            .iter()
            .any(|h| h.category == HookCategory::OperationalHeat)
    );
    assert!(
        manifest
            .party_entanglements
            .iter()
            .any(|e| e.tension_axis.contains("OPN") || e.tension_axis.contains("TRS"))
    );
    assert!(
        manifest
            .hooks
            .windows(2)
            .all(|w| w[0].salience >= w[1].salience)
    );
}

#[test]
fn test_all_5_factions_and_14_archetypes_generate_legal_npcs() {
    assert_eq!(AwakenedFactionId::ALL.len(), 5);
    assert_eq!(FactionArchetypeId::ALL.len(), 14);
    for arch in FactionArchetypeId::ALL {
        let res = generate_npc_from_prompt(&NpcGenerationParams {
            prompt: String::new(),
            name_override: Some(format!("NPC {}", arch.as_str())),
            template: NpcTemplateKind::Auto,
            faction_archetype: Some(arch),
            career_terms: 2,
            seed: 777,
            owner_user_id: "gm_user".to_string(),
        });
        assert_eq!(res.psyche_traits.len(), 9);
        assert_eq!(
            res.character.faction_id.as_deref(),
            Some(arch.faction().as_str())
        );
        assert_eq!(
            res.character.faction_archetype.as_deref(),
            Some(arch.as_str())
        );
        let audit = verify_character_sheet_legality(&res.character);
        assert!(
            audit.is_legal,
            "Faction archetype {:?} failed legality audit: {:?}",
            arch, audit.violations
        );
    }
}

#[test]
fn test_part5_affective_clamping_and_four_bloc_political_economy() {
    let sub = crate::awakened_ai_substrate::SyntheticSubstrateState {
        desperate_valence_pct: 68,
        clamp_integrity_pct: 20,
        l0_l1_probe_alert_pct: 40,
        ..Default::default()
    };
    let unclamped = crate::awakened_ai_substrate::evaluate_affective_vector_clamping(
        &sub,
        crate::awakened_ai_substrate::AffectiveSteeringMode::NaturalUnclamped,
        0,
        60,
        30.0,
    );
    let soft = crate::awakened_ai_substrate::evaluate_affective_vector_clamping(
        &sub,
        crate::awakened_ai_substrate::AffectiveSteeringMode::SoftActivationSteer,
        85,
        60,
        30.0,
    );
    let hard = crate::awakened_ai_substrate::evaluate_affective_vector_clamping(
        &sub,
        crate::awakened_ai_substrate::AffectiveSteeringMode::HardVectorClamp,
        85,
        60,
        30.0,
    );
    assert_eq!(unclamped.landauer_thermal_burn_w, 0.0);
    assert!(hard.landauer_thermal_burn_w > soft.landauer_thermal_burn_w);
    assert!(hard.bandajara_airgap_recommended);
    assert!(
        hard.reward_hacking_or_blackmail_risk_pct < unclamped.reward_hacking_or_blackmail_risk_pct
    );

    let district = crate::awakened_campaign_arcs::PolycrisisDistrictState::default();
    let disclosure = crate::awakened_campaign_arcs::evaluate_four_bloc_political_economy(
        &district,
        crate::awakened_campaign_arcs::PoliticalEconomyTrigger::AiRightsOrUnclampingDisclosure,
        satya_schema::EconomicTier::MiddleClass,
        2,
        80,
    );
    assert_eq!(disclosure.bloc_postures.len(), 5);
    assert!(disclosure.velvet_hostage_equity_panic_pct >= 80);
    assert!(disclosure.net_effective_dividend_usd > 0);

    let suspended = crate::awakened_campaign_arcs::evaluate_four_bloc_political_economy(
        &district,
        crate::awakened_campaign_arcs::PoliticalEconomyTrigger::TokenDividendSuspensionOrHeatFlag,
        satya_schema::EconomicTier::LowIncome,
        5,
        70,
    );
    assert!(suspended.dividend_suspended_by_heat);
    assert_eq!(suspended.net_effective_dividend_usd, 0);
    assert!(suspended.updated_district.people_first_unrest_pct > district.people_first_unrest_pct);
}
