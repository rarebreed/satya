use satya_schema::CharacterSheetRecord;

use crate::character_audit::{
    aptitudes_to_record, characteristics_to_record, verify_character_sheet_legality,
};
use crate::characteristics::{
    CharacterAptitudes, CharacterCharacteristics, CharacteristicCategory, CharacteristicCode,
    psyche_pole_descriptor,
};
use crate::combat::MoraleState;
use crate::economy_gear::{DebtTier, preview_next_skill_step_sp_with_hobbies};
use crate::lifepath_catalog::{CareerEchelonTier, CareerFundingMode, CareerTermTrack, HighSchoolTrack, Stage4Path};
use crate::lifepath_economy::{
    CareerTermConfig, LifepathBuildConfig, compute_lifepath_baseline_from_config,
    compute_skill_allocation_sp_ledger,
};
use crate::lifepath_hobbies_events::{
    FormativeLifeEventId, LifeEventChoiceBranch, PersonalHobbyId, resolve_formative_life_event,
};
use crate::psyche_social::{
    CombatMotivationKind, EmotionalTriggerKind, PsycheProfile, PsycheTraitId,
    evaluate_combat_motivation_modifier, evaluate_emotional_trigger,
};
use crate::skills_trees::lookup_skill_node;
use crate::social_combat::{
    SocialArenaKind, SocialArgumentVector, compute_argument_psyche_modifier,
    evaluate_social_standing_modifier,
};
use crate::walkthroughs::build_linnea_cho_lifepath;

#[test]
fn test_expanded_35_characteristics_and_9_psyche_traits() {
    assert_eq!(CharacteristicCode::ALL.len(), 35);
    assert_eq!(CharacteristicCode::SOCIAL_ALL.len(), 6);
    assert_eq!(CharacteristicCode::PSYCHE_ALL.len(), 9);
    assert_eq!(PsycheTraitId::ALL.len(), 9);
    assert_eq!(
        CharacteristicCode::SAV.category(),
        CharacteristicCategory::Social
    );
    assert_eq!(
        CharacteristicCode::BEA.category(),
        CharacteristicCategory::Social
    );
    assert_eq!(
        CharacteristicCode::WRM.category(),
        CharacteristicCategory::Social
    );
    assert_eq!(CharacteristicCode::from_code_str("POISE"), Some(CharacteristicCode::BEA));
    assert_eq!(
        CharacteristicCode::TRS.category(),
        CharacteristicCategory::Psyche
    );
    assert_eq!(
        CharacteristicCode::CPS.category(),
        CharacteristicCategory::Psyche
    );

    let defaults = CharacterCharacteristics::default();
    assert_eq!(defaults.sav, 10);
    assert_eq!(defaults.bea, 10);
    assert_eq!(defaults.wrm, 10);
    assert_eq!(defaults.trs, 10);
    assert_eq!(defaults.cps, 10);
    assert_eq!(defaults.compute_derived(0).cmp_social_composure_threshold, 5);
    let high_bea = CharacterCharacteristics {
        dis: 14,
        foc: 14,
        bea: 16,
        ..Default::default()
    };
    assert_eq!(high_bea.compute_derived(0).cmp_social_composure_threshold, 7);
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
    assert_eq!(linnea.socioeconomic.institutional_status, 4);
    assert_eq!(linnea.socioeconomic.street_cred, 3);
    assert_eq!(linnea.socioeconomic.debt_tier, DebtTier::Manageable);
}

#[test]
fn test_social_standing_psyche_leverage_and_arena_inversion() {
    let high_cnf = PsycheProfile {
        cnf: 15,
        ego: 11,
        ..PsycheProfile::default()
    };
    let low_cnf = PsycheProfile {
        cnf: 4,
        ..PsycheProfile::default()
    };
    let low_trs = PsycheProfile {
        trs: 4,
        ..PsycheProfile::default()
    };

    let inst_eval = evaluate_social_standing_modifier(
        9,
        0,
        3,
        2,
        &high_cnf,
        SocialArenaKind::InstitutionalFormal,
    );
    assert!(inst_eval.psyche_amplified);
    assert_eq!(inst_eval.delta_ext, -6);

    let mav_eval = evaluate_social_standing_modifier(
        9,
        0,
        3,
        2,
        &low_cnf,
        SocialArenaKind::InstitutionalFormal,
    );
    assert!(mav_eval.inverted_backlash);
    assert!(mav_eval.delta_ext > 0);

    let street_suit =
        evaluate_social_standing_modifier(8, 0, 2, 3, &low_cnf, SocialArenaKind::StreetUnderworld);
    assert!(street_suit.inverted_backlash);
    assert!(street_suit.delta_ext >= 4);

    let grassroots_suit = evaluate_social_standing_modifier(
        9,
        0,
        3,
        3,
        &low_trs,
        SocialArenaKind::GrassrootsCommunity,
    );
    assert!(grassroots_suit.inverted_backlash);
    assert!(grassroots_suit.delta_ext >= 5);
}

#[test]
fn test_bidirectional_lifepath_mobility_debt_and_echelons() {
    let chars = CharacterCharacteristics {
        ana: 13,
        mem: 12,
        dis: 12,
        foc: 12,
        ins: 12,
        elo: 13,
        bea: 12,
        sav: 12,
        for_stat: 11,
        fit: 11,
        hlt: 11,
        ..Default::default()
    };
    let apts = CharacterAptitudes {
        apt_qnt: 1,
        ..Default::default()
    };

    // 1. Upward GI-Bill Mobility: Working Poor -> Military -> STEM University -> Corporate Exec
    let gi_bill_cfg = LifepathBuildConfig {
        origin_roll_2d20: 10, // Working Poor (wealth=3, status=3)
        stage4_path_override: Some(Stage4Path::Path4AHighSchool),
        stage4a_hs_track: HighSchoolTrack::StandardCollegePrep,
        stage4a_hs_override: Some(1),
        career_terms: vec![
            CareerTermConfig {
                track: CareerTermTrack::MilitaryCombatArmsSpecOps,
                override_successes: Some(1),
                ..Default::default()
            },
            CareerTermConfig {
                track: CareerTermTrack::UniversityUndergraduateStem,
                override_successes: Some(2),
                ..Default::default()
            },
            CareerTermConfig {
                track: CareerTermTrack::CorporateManagementFinancePolicy,
                override_successes: Some(2),
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    let upward = compute_lifepath_baseline_from_config(&gi_bill_cfg, &chars, &apts);
    assert_eq!(
        upward.term_entry_evals[0].resolved_echelon,
        CareerEchelonTier::EntryRank
    );
    assert_eq!(
        upward.term_entry_evals[1].funding_mode,
        CareerFundingMode::GiBillServiceFunded
    );
    assert_eq!(upward.socioeconomic.debt_tier, DebtTier::None);
    assert!(upward.socioeconomic.wealth_tier >= 7);
    assert!(upward.socioeconomic.institutional_status >= 6);
    assert_eq!(
        upward.term_entry_evals[2].resolved_echelon,
        CareerEchelonTier::SeniorLeadership
    );

    // 1b. Corporate EntryRank vs MidSpecialist echelon check
    let entry_corp = compute_lifepath_baseline_from_config(
        &LifepathBuildConfig {
            origin_roll_2d20: 18,
            stage4_path_override: Some(Stage4Path::Path4AHighSchool),
            career_terms: vec![CareerTermConfig {
                track: CareerTermTrack::CorporateManagementFinancePolicy,
                override_successes: Some(1),
                ..Default::default()
            }],
            ..Default::default()
        },
        &chars,
        &apts,
    );
    assert_eq!(
        entry_corp.term_entry_evals[0].resolved_echelon,
        CareerEchelonTier::EntryRank
    );

    // 2. Debt-Funded Medical Entry: Working Poor -> Med School without GI Bill or Honors
    let low_study_chars = CharacterCharacteristics {
        dis: 10,
        mem: 10,
        ana: 10,
        ..Default::default()
    };
    let med_debt_cfg = LifepathBuildConfig {
        origin_roll_2d20: 10,
        stage4_path_override: Some(Stage4Path::Path4AHighSchool),
        stage4a_hs_override: Some(0),
        career_terms: vec![CareerTermConfig {
            track: CareerTermTrack::MedicalSchoolAndResidency,
            override_successes: Some(1),
            ..Default::default()
        }],
        ..Default::default()
    };
    let indebted =
        compute_lifepath_baseline_from_config(&med_debt_cfg, &low_study_chars, &CharacterAptitudes::default());
    assert_eq!(
        indebted.term_entry_evals[0].funding_mode,
        CareerFundingMode::DebtFunded
    );
    assert_eq!(indebted.socioeconomic.debt_tier, DebtTier::Manageable);
    assert!(indebted.term_entry_evals[0].entry_tn_mod >= 2);

    // 3. Downward Mobility & 2035 Shock: Wealthy Elite -> Corporate + AI Layoff + Grassroots NGO
    let downward_cfg = LifepathBuildConfig {
        origin_roll_2d20: 40, // Wealthy Elite (wealth=9, status=9)
        stage4_path_override: Some(Stage4Path::Path4AHighSchool),
        career_terms: vec![
            CareerTermConfig {
                track: CareerTermTrack::CorporateManagementFinancePolicy,
                override_successes: Some(1),
                life_event_roll_2d20: Some(15), // AI Layoff Shock (-1 W, -1 S, +1 Debt)
                ..Default::default()
            },
            CareerTermConfig {
                track: CareerTermTrack::GrassrootsActivismHumanitarianNgo,
                override_successes: Some(1),
                life_event_roll_2d20: Some(35), // Whistleblower Crossroads (-1 W, -2 S, +1 Cred, +1 Heat)
                life_event_choice: LifeEventChoiceBranch::BranchB,
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    let downward = compute_lifepath_baseline_from_config(&downward_cfg, &chars, &apts);
    assert!(downward.socioeconomic.wealth_tier < 9);
    assert!(downward.socioeconomic.institutional_status < 9);
    assert!(downward.socioeconomic.street_cred >= 2);
    assert!(downward.operational_heat >= 1);
    assert!(downward.term_entry_evals[1].entry_tn_mod >= 2);
    assert!(downward.socioeconomic.earned_contacts.len() >= 3);

    // 4. Syndicate White-Collar Launderer Echelon vs. Street Soldier
    let launderer_cfg = LifepathBuildConfig {
        origin_roll_2d20: 36, // Affluent Prep (wealth=8, not frozen)
        stage4_path_override: Some(Stage4Path::Path4AHighSchool),
        career_terms: vec![CareerTermConfig {
            track: CareerTermTrack::OrganizedCrimeCartelSyndicate,
            override_successes: Some(1),
            ..Default::default()
        }],
        ..Default::default()
    };
    let launderer = compute_lifepath_baseline_from_config(&launderer_cfg, &chars, &apts);
    assert_eq!(
        launderer.term_entry_evals[0].resolved_echelon,
        CareerEchelonTier::SeniorLeadership
    );
}
