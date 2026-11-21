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
use crate::lifepath_catalog::CareerTermTrack;
use crate::lifepath_economy::{
    LifepathBuildConfig, compute_lifepath_baseline_from_config, compute_skill_allocation_sp_ledger,
};
use crate::lifepath_hobbies_events::{
    FormativeLifeEventId, LifeEventChoiceBranch, PersonalHobbyId, resolve_formative_life_event,
};
use crate::psyche_social::{
    CombatMotivationKind, EmotionalTriggerKind, PsycheProfile, PsycheTraitId,
    evaluate_combat_motivation_modifier, evaluate_emotional_trigger,
};
use crate::skills_trees::lookup_skill_node;
use crate::social_combat::{SocialArgumentVector, compute_argument_psyche_modifier};
use crate::walkthroughs::build_linnea_cho_lifepath;

#[test]
fn test_expanded_32_characteristics_and_9_psyche_traits() {
    assert_eq!(CharacteristicCode::ALL.len(), 32);
    assert_eq!(CharacteristicCode::PSYCHE_ALL.len(), 9);
    assert_eq!(PsycheTraitId::ALL.len(), 9);
    assert_eq!(
        CharacteristicCode::TRS.category(),
        CharacteristicCategory::Psyche
    );
    assert_eq!(
        CharacteristicCode::CPS.category(),
        CharacteristicCategory::Psyche
    );

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
    assert_eq!(
        silent.event_id,
        FormativeLifeEventId::WhistleblowerCrossroads
    );
    assert_eq!(silent.sqm_delta, 1);
    assert_eq!(whistle.heat_delta, 1);
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
