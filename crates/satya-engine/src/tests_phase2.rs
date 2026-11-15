use std::collections::BTreeMap;

use satya_schema::CharacterSheetRecord;

use crate::character_audit::{
    aptitudes_to_record, characteristics_to_record, verify_character_sheet_legality,
};
use crate::characteristics::{
    CharacterAptitudes, CharacterCharacteristics, CharacteristicCategory, CharacteristicCode,
    psyche_pole_descriptor,
};
use crate::combat::MoraleState;
use crate::economy_gear::preview_next_skill_step_sp_with_hobbies;
use crate::faction_templates::{AwakenedFactionId, FactionArchetypeId};
use crate::hook_weaver::{HookCategory, compile_party_hooks, derive_npc_motivational_profile};
use crate::lifepath_catalog::CareerTermTrack;
use crate::lifepath_economy::{
    LifepathBuildConfig, compute_lifepath_baseline_from_config, compute_skill_allocation_sp_ledger,
};
use crate::lifepath_hobbies_events::{
    FormativeLifeEventId, LifeEventChoiceBranch, PersonalHobbyId, resolve_formative_life_event,
};
use crate::npc_generator::{NpcGenerationParams, NpcTemplateKind, generate_npc_from_prompt};
use crate::psyche_social::{
    CombatMotivationKind, EmotionalTriggerKind, PsycheProfile, PsycheTraitId,
    evaluate_combat_motivation_modifier, evaluate_emotional_trigger,
};
use crate::skills_trees::lookup_skill_node;
use crate::social_combat::{SocialArgumentVector, compute_argument_psyche_modifier};
use crate::walkthroughs::build_linnea_cho_lifepath;

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
        profile.negotiation_lever.contains("BLG=16")
            || profile.negotiation_lever.contains("SLF=5")
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
fn test_expanded_32_characteristics_and_9_psyche_traits() {
    assert_eq!(CharacteristicCode::ALL.len(), 32);
    assert_eq!(CharacteristicCode::PSYCHE_ALL.len(), 9);
    assert_eq!(PsycheTraitId::ALL.len(), 9);
    assert_eq!(CharacteristicCode::TRS.category(), CharacteristicCategory::Psyche);
    assert_eq!(CharacteristicCode::CPS.category(), CharacteristicCategory::Psyche);

    let defaults = CharacterCharacteristics::default();
    assert_eq!(defaults.trs, 10);
    assert_eq!(defaults.cps, 10);
    assert!(psyche_pole_descriptor(CharacteristicCode::TRS, 17).contains("Trust"));
    assert!(psyche_pole_descriptor(CharacteristicCode::CPS, 4).contains("Ruthless"));
    assert!(psyche_pole_descriptor(CharacteristicCode::CPS, 10).contains("Humane"));
}

#[test]
fn test_trust_and_compassion_morale_triggers_and_social_wedges() {
    let guardian = PsycheProfile {
        cps: 16,
        trs: 16,
        ..PsycheProfile::default()
    };
    let callous = PsycheProfile {
        cps: 4,
        trs: 4,
        ..PsycheProfile::default()
    };

    let guard_mod = evaluate_combat_motivation_modifier(
        &guardian,
        CombatMotivationKind::ShieldingInnocentsSanctityOfLife,
    );
    let callous_mod = evaluate_combat_motivation_modifier(
        &callous,
        CombatMotivationKind::ShieldingInnocentsSanctityOfLife,
    );
    assert_eq!(guard_mod.delta_mot, 2);
    assert_eq!(callous_mod.delta_mot, -1);

    let betrayed_trusting = evaluate_emotional_trigger(
        &guardian,
        11,
        MoraleState::Steady,
        0,
        EmotionalTriggerKind::BetrayedByTrustedAlly,
        &[(10, 10)],
    );
    let betrayed_cynical = evaluate_emotional_trigger(
        &callous,
        11,
        MoraleState::Steady,
        0,
        EmotionalTriggerKind::BetrayedByTrustedAlly,
        &[(10, 10)],
    );
    assert!(betrayed_trusting.susceptible);
    assert_eq!(betrayed_trusting.delta_ext, 4);
    assert!(!betrayed_cynical.susceptible);
    assert_eq!(betrayed_cynical.delta_ext, 0);

    let innocent_guardian = evaluate_emotional_trigger(
        &guardian,
        11,
        MoraleState::Steady,
        0,
        EmotionalTriggerKind::SeeingInnocentSuffering,
        &[(10, 10)],
    );
    let innocent_callous = evaluate_emotional_trigger(
        &callous,
        11,
        MoraleState::Steady,
        0,
        EmotionalTriggerKind::SeeingInnocentSuffering,
        &[(10, 10)],
    );
    assert!(innocent_guardian.susceptible);
    assert!(!innocent_callous.susceptible);

    let populist = PsycheProfile {
        blg: 16,
        cps: 13,
        opn: 5,
        trs: 4,
        ..PsycheProfile::default()
    };
    let wedge_eval = compute_argument_psyche_modifier(
        &populist,
        &[
            SocialArgumentVector::MoralSanctityWitnessedSufferingMercy {
                utilitarian_greater_good_triage: false,
            },
            SocialArgumentVector::TransparencyVulnerabilityGoodFaith {
                verifies_with_audit_or_proof: false,
            },
        ],
    );
    assert!(wedge_eval.resonance_delta_ext < 0);
    assert!(wedge_eval.clash_delta_ext > 0);
    assert!(wedge_eval.triggers_dual_psyche_wedge);
}

#[test]
fn test_14_career_tracks_and_12_hobbies_club_discount() {
    assert_eq!(CareerTermTrack::ALL.len(), 14);
    for track in CareerTermTrack::ALL {
        assert!(!track.recommended_skills().is_empty());
        for &sk in track.recommended_skills() {
            assert!(
                lookup_skill_node(sk).is_some(),
                "Career track {:?} referenced unknown skill '{sk}'",
                track
            );
        }
    }

    assert_eq!(PersonalHobbyId::ALL.len(), 12);
    for hobby in PersonalHobbyId::ALL {
        let spec = hobby.spec();
        assert!(!spec.signature_skill_ids.is_empty());
        for &sk in spec.signature_skill_ids {
            assert!(
                lookup_skill_node(sk).is_some(),
                "Hobby {:?} referenced unknown skill '{sk}'",
                hobby
            );
        }
    }

    let cfg = LifepathBuildConfig::default();
    let chars = CharacterCharacteristics::default();
    let aptitudes = CharacterAptitudes::default();
    let baseline = compute_lifepath_baseline_from_config(&cfg, &chars, &aptitudes);
    let ledger = compute_skill_allocation_sp_ledger(
        &baseline,
        &cfg,
        &chars,
        &aptitudes,
        &baseline.baseline_skills,
    );
    let club = [PersonalHobbyId::DebateTheaterImprov];
    let (plain_n2, _) = preview_next_skill_step_sp_with_hobbies(
        "writing_composition",
        2,
        &cfg,
        &[],
        &ledger,
        &chars,
        &aptitudes,
    );
    let (club_n2, _) = preview_next_skill_step_sp_with_hobbies(
        "writing_composition",
        2,
        &cfg,
        &club,
        &ledger,
        &chars,
        &aptitudes,
    );
    assert!(
        club_n2.training_mode_multiplier < plain_n2.training_mode_multiplier,
        "Expected 0.85x hobby discount at N=2"
    );
    assert!(club_n2.final_sp_cost <= plain_n2.final_sp_cost);
    assert!(club_n2.final_sp_cost >= 3);

    let (plain_n4, _) = preview_next_skill_step_sp_with_hobbies(
        "writing_composition",
        4,
        &cfg,
        &[],
        &ledger,
        &chars,
        &aptitudes,
    );
    let (club_n4, _) = preview_next_skill_step_sp_with_hobbies(
        "writing_composition",
        4,
        &cfg,
        &club,
        &ledger,
        &chars,
        &aptitudes,
    );
    assert_eq!(club_n4.final_sp_cost, plain_n4.final_sp_cost);
}

#[test]
fn test_12_formative_life_events_and_branches() {
    assert_eq!(FormativeLifeEventId::ALL.len(), 12);
    for roll in 2..=40 {
        let ev = FormativeLifeEventId::from_roll_2d20(roll);
        let res = resolve_formative_life_event(roll, LifeEventChoiceBranch::DefaultPath, true);
        assert_eq!(res.event_id, ev);
        for &sk in res.recommended_skill_ids {
            assert!(
                lookup_skill_node(sk).is_some(),
                "Life event {:?} referenced unknown skill '{sk}'",
                ev
            );
        }
    }

    let silent = resolve_formative_life_event(35, LifeEventChoiceBranch::BranchA, true);
    let whistle = resolve_formative_life_event(35, LifeEventChoiceBranch::BranchB, true);
    assert_eq!(silent.event_id, FormativeLifeEventId::WhistleblowerCrossroads);
    assert_eq!(silent.sqm_delta, 1);
    assert_eq!(whistle.heat_delta, 1);
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
fn test_linnea_cho_lifepath_passes_legality_audit() {
    let linnea = build_linnea_cho_lifepath();
    assert_eq!(linnea.characteristics.cps, 17);
    assert_eq!(linnea.characteristics.trs, 12);
    let rec = CharacterSheetRecord {
        character_id: "linnea_cho".to_string(),
        character_name: linnea.name.clone(),
        age_years: linnea.age_years as i32,
        origin_label: linnea.origin.label.to_string(),
        campaign_sqm: i32::from(linnea.campaign_sqm),
        operational_heat: i32::from(linnea.operational_heat),
        granted_access_tags: linnea
            .granted_access_tags
            .iter()
            .map(|t| t.as_str().to_string())
            .collect(),
        stage_history: linnea.stage_history.clone(),
        characteristics: characteristics_to_record(&linnea.characteristics),
        aptitudes: aptitudes_to_record(&linnea.aptitudes),
        skills: linnea
            .skills
            .iter()
            .map(|(k, &v)| (k.clone(), v as i32))
            .collect(),
        faction_id: Some("SATYANG".to_string()),
        faction_archetype: Some("SATYANG_MANJUSRI".to_string()),
        ..CharacterSheetRecord::default()
    };
    let audit = verify_character_sheet_legality(&rec);
    assert!(
        audit.is_legal,
        "Linnea Cho failed legality audit: {:?}",
        audit.violations
    );
}
