//! Domain unit tests for characteristics, skills trees, action economy, psyche/social, and health/survival.

use crate::*;

#[test]
fn test_spectrum_and_population_tiers() {
    assert_eq!(spectrum_to_controlling_scale(50.0), 10);
    assert_eq!(spectrum_to_controlling_scale(0.0), 1);
    assert_eq!(spectrum_to_controlling_scale(100.0), 20);
    assert!((controlling_scale_to_spectrum(10) - 50.0).abs() < 1e-6);

    assert_eq!(
        PopulationTier::from_controlling_scale(2),
        PopulationTier::SeverelyImpaired
    );
    assert_eq!(
        PopulationTier::from_controlling_scale(10),
        PopulationTier::HumanMedian
    );
    assert_eq!(
        PopulationTier::from_controlling_scale(18),
        PopulationTier::PeakUnaugmented
    );
    assert_eq!(
        PopulationTier::from_controlling_scale(20),
        PopulationTier::AugmentedApex
    );
    assert_eq!(PopulationTier::base_tn(10), 21);
    assert_eq!(PopulationTier::base_tn(15), 16);
}

#[test]
fn test_all_10_derived_characteristics_and_aptitudes() {
    let chars = CharacterCharacteristics {
        spd: 14,
        for_stat: 12,
        kin: 14,
        hlt: 13,
        fit: 12,
        mas_kg: 84.0,
        hgt_m: 1.90,
        ana: 15,
        cre: 12,
        foc: 13,
        itu: 14,
        ins: 12,
        sig: 15,
        hea: 11,
        dis: 14,
        ..Default::default()
    };

    let derived = chars.compute_derived(2);
    // POW = (12 * 14) / 10 = 16.8
    assert!((derived.pow_striking_power - 16.8).abs() < 1e-6);
    // MMF = (84 / 70) * 14 = 16.8
    assert!((derived.mmf_momentum_factor - 16.8).abs() < 1e-6);
    // Height edge = (1.90 - 1.75)/0.15 = +1, KIN edge = (14 - 10)/4 = +1, weapon = +2 -> 4
    assert_eq!(derived.height_reach_edge, 1);
    assert_eq!(derived.melee_reach_index, 4);
    // D_cog = floor((max(15, 12) + 13) / 2) = 14
    assert_eq!(derived.d_cog_quick_thinking, 14);
    // OODA = floor((14 + 12 + 14 + 14) / 4) = 13
    assert_eq!(derived.ooda_initiative, 13);
    // AWR = floor((max(15, 11) + 13 + 14) / 3) = 14
    assert_eq!(derived.awr_general_awareness, 14);
    // FT = 12 + floor(14 / 2) = 19
    assert_eq!(derived.ft_fatigue_threshold, 19);
    // WSR = 13 + floor(14 / 2) = 20
    assert_eq!(derived.wsr_wound_shock_resistance, 20);
    // CMP = floor((14 + 13) / 4) = 6
    assert_eq!(derived.cmp_social_composure_threshold, 6);

    // Mateo Vance C_study check: DIS 11, ANA 13, APT-QNT +3 -> floor((11+13)/2) + 3 = 15
    assert_eq!(compute_academic_study_controller(11, 13, 3), 15);

    // Aptitude training time multipliers: +2 -> 0.70 (-30%), -2 -> 1.40 (+40%)
    assert!((compute_aptitude_training_time_multiplier(2) - 0.70).abs() < 1e-6);
    assert!((compute_aptitude_training_time_multiplier(-2) - 1.40).abs() < 1e-6);
    assert_eq!(compute_aptitude_min_c_relief(2), 1);
    assert_eq!(compute_aptitude_min_c_relief(1), 0);
}

#[test]
fn test_characteristic_roll_methods_height_mass_and_aptitude_pool() {
    // Method 1: 2d4 + 3d5 drop lowest d5
    // Sequence: d4=[3, 4], d5=[2, 5, 4] -> drops 2 (index 0), total = 3 + 4 + 5 + 4 = 16
    let seq = [3, 4, 2, 5, 4];
    let mut idx = 0usize;
    let r1 = roll_characteristic_method1(&mut |_sides| {
        let v = seq[idx % seq.len()];
        idx += 1;
        v
    });
    assert_eq!(r1.d4_rolls, [3, 4]);
    assert_eq!(r1.d5_rolls, vec![2, 5, 4]);
    assert_eq!(r1.dropped_d5_index, Some(0));
    assert_eq!(r1.total, 16);

    // Method 1 Physical Pool: 6 rolls, 6 kept
    let mut counter = 1i32;
    let phys_m1 = roll_characteristic_group_pool(
        CharacteristicCategory::Physical,
        CharacteristicRollMethod::Method1DropLowestD5,
        &mut |sides| {
            let v = ((counter - 1) % sides) + 1;
            counter += 1;
            v
        },
    );
    assert_eq!(phys_m1.raw_rolls.len(), 6);
    assert_eq!(phys_m1.kept_values.len(), 6);
    assert!(phys_m1.kept_values.iter().all(|&v| (4..=18).contains(&v)));

    // Method 1 Social Pool: 3 rolls, 3 kept
    let soc_m1 = roll_characteristic_group_pool(
        CharacteristicCategory::Social,
        CharacteristicRollMethod::Method1DropLowestD5,
        &mut |sides| {
            let v = ((counter - 1) % sides) + 1;
            counter += 1;
            v
        },
    );
    assert_eq!(soc_m1.raw_rolls.len(), 3);
    assert_eq!(soc_m1.kept_values.len(), 3);

    // Method 2 Physical Pool: 8 rolls of 2d4+2d5, drops 2 lowest -> keeps 6 highest
    let phys_m2 = roll_characteristic_group_pool(
        CharacteristicCategory::Physical,
        CharacteristicRollMethod::Method2PoolKeepHighest,
        &mut |sides| {
            let v = ((counter - 1) % sides) + 1;
            counter += 1;
            v
        },
    );
    assert_eq!(phys_m2.raw_rolls.len(), 8);
    assert_eq!(
        phys_m2
            .raw_rolls
            .iter()
            .filter(|r| r.dropped_from_pool)
            .count(),
        2
    );
    assert_eq!(phys_m2.kept_values.len(), 6);

    // Method 2 Social Pool: 4 rolls of 2d4+2d5, drops 1 lowest -> keeps 3 highest
    let soc_m2 = roll_characteristic_group_pool(
        CharacteristicCategory::Social,
        CharacteristicRollMethod::Method2PoolKeepHighest,
        &mut |sides| {
            let v = ((counter - 1) % sides) + 1;
            counter += 1;
            v
        },
    );
    assert_eq!(soc_m2.raw_rolls.len(), 4);
    assert_eq!(
        soc_m2
            .raw_rolls
            .iter()
            .filter(|r| r.dropped_from_pool)
            .count(),
        1
    );
    assert_eq!(soc_m2.kept_values.len(), 3);

    // Correlated Height & Mass around mean (roll 11 -> 1.72m, FOR 10 -> BMI 23.0 -> 68.0 kg)
    let (hgt, bmi, mas) = compute_correlated_height_mass(11, 11, 10);
    assert!((hgt - 1.72).abs() < 1e-6);
    assert!((bmi - 23.0).abs() < 1e-6);
    assert!((mas - 68.0).abs() < 1e-6);

    // Aptitude 3d6 table & 11-roll drop-2-lowest pool
    assert_eq!(aptitude_from_3d6_sum(3), -2);
    assert_eq!(aptitude_from_3d6_sum(4), -2);
    assert_eq!(aptitude_from_3d6_sum(6), -1);
    assert_eq!(aptitude_from_3d6_sum(10), 0);
    assert_eq!(aptitude_from_3d6_sum(13), 0);
    assert_eq!(aptitude_from_3d6_sum(14), 1);
    assert_eq!(aptitude_from_3d6_sum(17), 2);
    assert_eq!(aptitude_from_3d6_sum(18), 3);

    let apt_pool = roll_aptitude_pool_11_drop_2(&mut |sides| {
        let v = ((counter - 1) % sides) + 1;
        counter += 1;
        v
    });
    assert_eq!(apt_pool.raw_rolls.len(), 11);
    assert_eq!(
        apt_pool
            .raw_rolls
            .iter()
            .filter(|r| r.dropped_from_pool)
            .count(),
        2
    );
    assert_eq!(apt_pool.kept_values.len(), 9);
    assert!(apt_pool.kept_values.iter().all(|&v| (-2..=3).contains(&v)));
}

#[test]
fn test_universal_childhood_talents_and_8_trees() {
    let talents = initialize_universal_talents();
    assert_eq!(talents.get("native_language_speech"), Some(&2));
    assert_eq!(talents.get("counting_basic_numeracy"), Some(&1));
    assert_eq!(talents.get("manual_dexterity_tinkering"), Some(&1));
    assert_eq!(talents.get("hygiene_basic_self_care"), Some(&1));
    assert_eq!(talents.get("charm_rapport"), Some(&1));
    assert_eq!(talents.get("brawling_unarmed_striking"), Some(&1));
    assert_eq!(talents.get("alertness_situational_awareness"), Some(&1));
    assert_eq!(talents.len(), 21);
    assert_eq!(CANONICAL_SKILL_TREES.len(), 129);
}

#[test]
fn test_foundation_ceiling_and_prereq_validation() {
    let mut skills = initialize_universal_talents();
    skills.insert("pistols_handguns".to_string(), 2);

    let chars = CharacterCharacteristics {
        for_stat: 11,
        dex: 13,
        ..Default::default()
    };
    let aptitudes = CharacterAptitudes::default();
    let access = vec![SkillAccessTier::Illicit];

    // Advancing Fully Automatic Weapons to N = 4 fits Foundation Ceiling (parent Pistols N = 2 -> max 2 + 2 = 4)
    let ok_res = validate_skill_advancement(
        "fully_automatic_burst_weapons",
        4,
        &skills,
        &chars,
        &aptitudes,
        &access,
        false,
    );
    assert!(ok_res.is_valid);

    // Attempting N = 5 violates Foundation Ceiling (5 > 2 + 2)
    let ceil_res = validate_skill_advancement(
        "fully_automatic_burst_weapons",
        5,
        &skills,
        &chars,
        &aptitudes,
        &access,
        false,
    );
    assert!(!ceil_res.is_valid);
    assert!(
        ceil_res
            .failures
            .iter()
            .any(|f| f.code == "FOUNDATION_CEILING_EXCEEDED")
    );
}

#[test]
fn test_tree_defaulting_and_no_default_block() {
    let mut skills = initialize_universal_talents();
    skills.insert("pistols_handguns".to_string(), 2);

    // Defaulting from Pistols (N=2) to Fully Automatic Weapons (N=0, defaultable=true) -> +4 Delta_ext
    let def = resolve_skill_tree_default("fully_automatic_burst_weapons", &skills);
    assert!(def.can_attempt);
    assert_eq!(def.effective_n, 2);
    assert_eq!(def.default_penalty_delta_ext, 4);

    // Sibling default: Pistols (N=2) shares TAL parent manual_dexterity_tinkering (N=1) with Long Guns -> prefers N=2 sibling!
    let sib_def = resolve_skill_tree_default("long_guns_rifles_carbines_shotguns", &skills);
    assert!(sib_def.can_attempt);
    assert_eq!(sib_def.source_skill_id.as_deref(), Some("pistols_handguns"));
    assert_eq!(sib_def.effective_n, 2);
    assert_eq!(sib_def.default_penalty_delta_ext, 4);

    // Trauma Surgery is [No Default] -> cannot be attempted at N=0
    let surg = resolve_skill_tree_default("trauma_orthopedic_general_surgery", &skills);
    assert!(!surg.can_attempt);
}

#[test]
fn test_skill_point_cost_formula_swimming_vs_helicopter() {
    let avg_chars = CharacterCharacteristics::default();
    let avg_apts = CharacterAptitudes::default();

    // 1. Swimming & Diving (Difficulty 1, PHY) at human average (C=10, APT=0, FOC=10, INS=10)
    let swim_n1_exp = compute_skill_step_sp_cost(
        "swimming_diving",
        1,
        SkillTrainingMode::Experience,
        &avg_chars,
        &avg_apts,
    );
    assert_eq!(swim_n1_exp.difficulty_rating, 1);
    assert_eq!(swim_n1_exp.final_sp_cost, 10);

    let swim_n1_formal = compute_skill_step_sp_cost(
        "swimming_diving",
        1,
        SkillTrainingMode::FormalTraining,
        &avg_chars,
        &avg_apts,
    );
    assert_eq!(swim_n1_formal.final_sp_cost, 9);

    // 2. Piloting (Rotary-Wing / Helicopter & VTOL) (Difficulty 5, PHY/KNO) at human average
    let heli_n1_formal = compute_skill_step_sp_cost(
        "piloting_rotary_wing_helicopter",
        1,
        SkillTrainingMode::FormalTraining,
        &avg_chars,
        &avg_apts,
    );
    let heli_n2_formal = compute_skill_step_sp_cost(
        "piloting_rotary_wing_helicopter",
        2,
        SkillTrainingMode::FormalTraining,
        &avg_chars,
        &avg_apts,
    );
    let heli_n3_formal = compute_skill_step_sp_cost(
        "piloting_rotary_wing_helicopter",
        3,
        SkillTrainingMode::FormalTraining,
        &avg_chars,
        &avg_apts,
    );
    assert_eq!(heli_n1_formal.difficulty_rating, 5);
    assert_eq!(heli_n1_formal.final_sp_cost, 21);
    assert_eq!(heli_n2_formal.final_sp_cost, 43);
    assert_eq!(heli_n3_formal.final_sp_cost, 64);

    // Unassisted experience for Helicopter Piloting is markedly more expensive (31 / 63 / 94 SP)
    let heli_n1_exp = compute_skill_step_sp_cost(
        "piloting_rotary_wing_helicopter",
        1,
        SkillTrainingMode::Experience,
        &avg_chars,
        &avg_apts,
    );
    assert_eq!(heli_n1_exp.final_sp_cost, 31);

    // 3. Gifted aviation student (KIN=15, DEX=14, FOC=14, INS=13, APT_KIN=+2)
    let gifted_chars = CharacterCharacteristics {
        kin: 15,
        dex: 14,
        foc: 14,
        ins: 13,
        ..Default::default()
    };
    let gifted_apts = CharacterAptitudes {
        apt_kin: 2,
        ..Default::default()
    };

    let heli_gifted_n1 = compute_skill_step_sp_cost(
        "piloting_rotary_wing_helicopter",
        1,
        SkillTrainingMode::FormalTraining,
        &gifted_chars,
        &gifted_apts,
    );
    assert!(heli_gifted_n1.final_sp_cost < heli_n1_formal.final_sp_cost);
    assert_eq!(heli_gifted_n1.final_sp_cost, 10);
}

#[test]
fn test_two_component_action_cost_and_reflex_compression() {
    // Worked check from §4.1 & §5.2-5.3:
    // Assess Tactical Weakness: base_tc_decide = 5, base_tc_act = 1 (Total 6).
    // Character with D_cog = 16 -> delta_tc_decide = -floor((16-10)/2) = -3.
    // Skill level N = 4 -> reflex compression = -1.
    // Effective TC_decide = max(1, 5 - 3 - 1) = 1.
    // Effective TC_act = 1. Total = 2 ticks!
    let params = ActionTimeFactorParams::default();
    let cost = evaluate_combat_action_cost(5, 1, 16, 4, true, 0, 0, 2, &params);
    assert_eq!(cost.delta_tc_decide, -3);
    assert_eq!(cost.skill_reflex_compression, 1);
    assert_eq!(cost.effective_tc_decide, 1);
    assert_eq!(cost.effective_tc_act, 1);
    assert_eq!(cost.effective_total_tc, 2);
}

#[test]
fn test_encumbrance_and_concussed_action_time_factor() {
    let params = ActionTimeFactorParams {
        fatigue_level: 1,
        wound_shock_index: 1.0,
        encumbrance_tier: EncumbranceTier::Tier4OverloadedStaggering,
        status_time_penalty: 0.25,
        is_concussed_or_dazed: true,
    };
    // Physical: 1.0 + 0.10 + 0.25 + 0.50 (Tier 4) + 0.25 (status) = 2.10
    assert!((params.physical_action_time_factor() - 2.10).abs() < 1e-6);
    // Cognitive (ignores physical encumbrance): 1.0 + 0.10 + 0.25 + 0.25 = 1.60
    assert!((params.cognitive_action_time_factor() - 1.60).abs() < 1e-6);
    // Concussed reduces D_cog by -4
    assert_eq!(params.effective_d_cog(14), 10);

    // §5.1 Compounding Fatigue 2 (+0.20) + Wound Shock 2 (+0.50) = 1.70x:
    // A 10-tick (3 decide + 7 act) strike at neutral stats costs 5 + 12 = 17 ticks!
    let compound = ActionTimeFactorParams {
        fatigue_level: 2,
        wound_shock_index: 2.0,
        encumbrance_tier: EncumbranceTier::Tier0Unencumbered,
        status_time_penalty: 0.0,
        is_concussed_or_dazed: false,
    };
    let strike_cost = evaluate_combat_action_cost(3, 7, 10, 0, false, 0, 0, 2, &compound);
    assert_eq!(strike_cost.effective_tc_decide, 5);
    assert_eq!(strike_cost.effective_tc_act, 12);
    assert_eq!(strike_cost.effective_total_tc, 17);
}

#[test]
fn test_starting_tick_and_interrupt_spoiling() {
    // OODA = 12, Awareness Margin = +2 -> Tick_start = max(1, 15 - 12 - 2) = 1
    assert_eq!(compute_starting_tick(12, 2, 0, 14, 12), 1);
    // Surprised (+10 ticks), D_cog = 14 (-2), DIS = 12 (-1) -> +7 freeze -> 1 + 7 = 8
    assert_eq!(compute_starting_tick(12, 2, 10, 14, 12), 8);

    // Heavy Power Strike (TC = 3 + 11 = 14) declared at Tick 2 -> completes at Tick 16.
    // Snap Shot lands at Tick 9 and causes Wound Shock (+2 delay) -> spoils the Heavy Strike!
    let params = ActionTimeFactorParams::default();
    let heavy_cost = evaluate_combat_action_cost(3, 11, 10, 2, true, 0, 0, 2, &params);
    let interrupt = evaluate_action_interrupt(2, &heavy_cost, 9, true, 2);
    assert!(interrupt.action_spoiled);
    assert!(interrupt.struck_during_wind_up);
    assert_eq!(interrupt.resulting_actor_tick, 11);
}

#[test]
fn test_canonical_action_catalog_and_stealth_perception() {
    assert_eq!(CANONICAL_ACTION_CATALOG.len(), 37);
    let speed_reload =
        lookup_canonical_action(CanonicalActionId::SpeedReloadDetachableMagDropEmpty);
    assert_eq!(speed_reload.base_tc_decide, 1);
    assert_eq!(speed_reload.base_tc_act, 5);
    assert_eq!(speed_reload.base_total_tc, 6);

    let infiltrator = CharacterCharacteristics {
        kin: 14,
        itu: 12,
        ..Default::default()
    };
    let observer = CharacterCharacteristics {
        sig: 13,
        hea: 11,
        olf: 10,
        itu: 12,
        foc: 13,
        ins: 11,
        ..Default::default()
    };
    let stealth = compute_stealth_and_perception_modifiers(&StealthExchangeParams {
        infiltrator,
        infiltrator_gait: StealthGait::CombatGlide,
        infiltrator_encumbrance: EncumbranceTier::Tier2TacticalFightingLoad,
        infiltrator_camouflage_bonus_delta_ext: 0,
        infiltrator_thermally_shielded: false,
        observer,
        observer_channel: SensoryChannel::VisualSigFoc,
        illumination: IlluminationCondition::MoonlightStarlight,
        countermeasure: VisualCountermeasure::Gen3PlusNvg,
    });
    // Infiltrator: floor((14+12)/2) = 13, Glide (+4) + Tier 2 (+2) = +6
    assert_eq!(stealth.infiltrator_controller_c, 13);
    assert_eq!(stealth.infiltrator_delta_ext, 6);
    // Observer: floor((13+13)/2) = 13, Moonlight with Gen3+ NVG = +1
    assert_eq!(stealth.observer_controller_c, 13);
    assert_eq!(stealth.observer_delta_ext, 1);
}

#[test]
fn test_psyche_friction_check_and_drift_threshold() {
    let params = PsycheFrictionCheckParams {
        trait_id: PsycheTraitId::Belonging,
        trait_score: 16,
        violated_pole: PsycheViolationPole::HighPole,
        dis: 12,
        has_sere_training: false,
        delta_int: 0,
        delta_ext: 0,
    };
    assert_eq!(psyche_deviation(16), 6);
    assert!(triggers_psyche_friction_check(
        16,
        PsycheViolationPole::HighPole
    ));

    let contest = build_psyche_friction_contest(&params);
    assert_eq!((contest.player.tn_eff(), contest.gm.tn_eff()), (19, 15));

    let clean = evaluate_psyche_friction_roll(&params, &[(12, 10)], (7, 7));
    assert!(clean.triggered);
    assert_eq!(
        clean.resolution_kind,
        PsycheFrictionResolutionKind::CleanOverride
    );
    assert_eq!(
        (clean.delta_int_penalty_gained, clean.drift_marks_gained),
        (0, 1)
    );

    let strained = evaluate_psyche_friction_roll(&params, &[(12, 10)], (9, 8));
    assert_eq!(
        strained.resolution_kind,
        PsycheFrictionResolutionKind::StrainedOverride
    );
    assert_eq!(
        (
            strained.delta_int_penalty_gained,
            strained.drift_marks_gained
        ),
        (2, 1)
    );

    let balk = evaluate_psyche_friction_roll(&params, &[(9, 8)], (9, 8));
    assert_eq!(
        balk.resolution_kind,
        PsycheFrictionResolutionKind::BalkOrFreeze
    );
    assert_eq!((balk.hesitation_ticks, balk.drift_marks_gained), (6, 0));

    assert_eq!(psyche_drift_threshold(12), 3);
    let d1 = apply_psyche_drift_mark(16, 2, 1, 12, None);
    assert!(d1.shifted && d1.new_score == 15 && d1.remaining_drift_marks == 0);
}

#[test]
fn test_conviction_surge_mechanics() {
    let profile = PsycheProfile {
        blg: 16,
        slf: 5,
        ..Default::default()
    };
    let surge = evaluate_conviction_surge(
        &profile,
        4,
        false,
        Some(ConvictionPrincipleArchetype::AltruisticOrTribalSacrifice),
    );
    assert!(surge.eligible && surge.ignores_fatigue_and_shock_time_penalty);
    assert_eq!(
        (
            surge.adjusted_delta_int,
            surge.surge_delta_under,
            surge.tactical_pool_bonus
        ),
        (-2, 4, 2)
    );
}

#[test]
fn test_marcus_vs_nadia_cognitive_dissonance_walkthrough() {
    let nadia = PsycheProfile {
        opn: 16,
        blg: 16,
        ..Default::default()
    };
    let state = SocialComposureState::new(11, 11);
    assert_eq!(state.max_cmp, 5);

    let read = evaluate_empathy_read(&nadia, 2);
    assert_eq!(read.revealed_traits.len(), 2);
    assert!(read.detects_psyche_wedge_tension);

    let vectors = [
        SocialArgumentVector::EmpiricalEvidenceLogicNovelIdeas,
        SocialArgumentVector::GroupConsensusLoyaltySharedIdentity {
            asks_to_betray_group: true,
        },
    ];
    let exchange = evaluate_social_exchange_wounds(&nadia, &state, &vectors, 2, None);
    assert_eq!(
        (
            exchange.argument_eval.resonance_delta_ext,
            exchange.argument_eval.clash_delta_ext,
            exchange.argument_eval.net_delta_ext
        ),
        (-6, 6, 0)
    );
    assert!(exchange.argument_eval.triggers_dual_psyche_wedge);
    assert!(exchange.wounds_inflicted.contains(&SocialWoundKind::Doubt));
    assert!(
        exchange
            .wounds_inflicted
            .contains(&SocialWoundKind::CognitiveDissonance)
    );
    assert_eq!(
        (
            exchange.updated_state.doubt_fractures,
            exchange.updated_state.dissonance_index,
            exchange.next_turn_dissonance_bleed_delta_int
        ),
        (1, 2, 4)
    );

    let denial = resolve_cognitive_dissonance(
        &exchange.updated_state,
        11,
        DissonanceResolutionPath::DogmaticDenial {
            dis_check_successes: 0,
        },
    );
    assert_eq!(denial.dogmatic_denial_tn, Some(24));

    let conversion = resolve_cognitive_dissonance(
        &exchange.updated_state,
        11,
        DissonanceResolutionPath::ConversionDefection {
            violated_trait: PsycheTraitId::Belonging,
        },
    );
    assert_eq!(conversion.updated_state.dissonance_index, 0);
    assert_eq!(
        conversion.psyche_drift_mark_awarded,
        Some(PsycheTraitId::Belonging)
    );
}

#[test]
fn test_hit_locations_and_called_shots() {
    let table = canonical_hit_location_table();
    assert_eq!(table.len(), 13);
    assert_eq!(resolve_hit_location_d20(1).id, HitLocationId::FootAnkle);
    assert_eq!(resolve_hit_location_d20(4).id, HitLocationId::KneeJoint);
    assert_eq!(
        resolve_hit_location_d20(14).id,
        HitLocationId::VitalThoracicHeartSpineAorta
    );
    assert_eq!(
        resolve_hit_location_d20(20).id,
        HitLocationId::HeadCraniumFaceJawEyes
    );
    assert_eq!(
        compute_called_shot_delta_ext(CalledShotZoneTier::BroadZone, 12, 9),
        0
    );
    assert_eq!(
        compute_called_shot_delta_ext(CalledShotZoneTier::BroadZone, 11, 9),
        2
    );
    assert_eq!(
        compute_called_shot_delta_ext(CalledShotZoneTier::SpecificSubZoneOrJoint, 14, 8),
        4
    );
    assert_eq!(
        compute_called_shot_delta_ext(CalledShotZoneTier::VitalPinpointOrArmorGap, 14, 8),
        8
    );
}

#[test]
fn test_fatigue_levels_and_asphyxiation() {
    let f0 = evaluate_fatigue_level(0);
    assert_eq!((f0.delta_int, f0.delta_under), (0, 6));
    assert!(f0.can_sprint);

    let f3 = evaluate_fatigue_level(3);
    assert_eq!(
        (
            f3.delta_int,
            f3.delta_under,
            f3.tactical_pool_drain,
            f3.spd_and_for_cap_penalty
        ),
        (6, 12, -6, 2)
    );
    assert!(f3.can_sprint);

    let f4 = evaluate_fatigue_level(4);
    assert!(!f4.can_sprint && !f4.is_unconscious);
    assert!(evaluate_fatigue_level(5).is_unconscious);

    let calm = evaluate_breath_holding_and_asphyxiation(10, 0, true, false, 400, false);
    assert_eq!(
        (
            calm.breath_holding_capacity_ticks,
            calm.fatigue_levels_gained
        ),
        (400, 0)
    );
    assert!(!calm.is_unconscious);

    let vig = evaluate_breath_holding_and_asphyxiation(10, 0, true, true, 260, false);
    assert_eq!(
        (vig.breath_holding_capacity_ticks, vig.fatigue_levels_gained),
        (160, 5)
    );
    assert!(vig.is_unconscious);

    let choke = evaluate_breath_holding_and_asphyxiation(10, 0, true, true, 28, true);
    assert!(choke.is_unconscious);
}

#[test]
fn test_wounds_fractures_hemorrhage_and_wsr_saves() {
    assert_eq!(classify_structural_wound_tier(0), None);
    assert_eq!(
        classify_structural_wound_tier(1),
        Some(StructuralWoundTier::Tier1Superficial)
    );
    assert_eq!(
        classify_structural_wound_tier(2),
        Some(StructuralWoundTier::Tier1Superficial)
    );
    assert_eq!(
        classify_structural_wound_tier(3),
        Some(StructuralWoundTier::Tier2Moderate)
    );
    assert_eq!(
        classify_structural_wound_tier(5),
        Some(StructuralWoundTier::Tier2Moderate)
    );
    assert_eq!(
        classify_structural_wound_tier(6),
        Some(StructuralWoundTier::Tier3SevereFracture)
    );
    assert_eq!(
        classify_structural_wound_tier(8),
        Some(StructuralWoundTier::Tier3SevereFracture)
    );
    assert_eq!(
        classify_structural_wound_tier(9),
        Some(StructuralWoundTier::Tier4CriticalMaiming)
    );
    assert_eq!(
        classify_structural_wound_tier(11),
        Some(StructuralWoundTier::Tier4CriticalMaiming)
    );
    assert_eq!(
        classify_structural_wound_tier(12),
        Some(StructuralWoundTier::Tier5CatastrophicLethal)
    );

    let (snapped, new_tier) = check_hairline_fracture_aggravation(7, 12, false);
    assert!(snapped && new_tier == StructuralWoundTier::Tier3SevereFracture);

    let bleed =
        evaluate_blood_loss_progression(0.0, &[HemorrhageGrade::Grade4ArterialSeverance], 20);
    assert!((bleed.total_units_lost - 5.0).abs() < 1e-6);
    assert_eq!(
        bleed.hypovolemic_class,
        HypovolemicShockClass::ClassIVLethalComa
    );
    assert!(bleed.immediate_unconsciousness_and_coma);
    assert_eq!(bleed.ticks_until_cardiac_arrest, Some(240));

    let save_win = evaluate_wound_shock_save(12, 12, 2, 2, 0, &[(12, 10), (12, 10)], (8, 7));
    assert!(save_win.stays_conscious && save_win.effective_wound_shock_index == 1);

    let save_fail = evaluate_wound_shock_save(10, 10, 1, 2, 0, &[(10, 9)], (10, 9));
    assert!(
        save_fail.stays_conscious
            && save_fail.stunned_ticks == 8
            && save_fail.drops_prone
            && save_fail.triggers_morale_check
    );

    let save_botch = evaluate_wound_shock_save(10, 10, 1, 2, 0, &[(5, 5)], (10, 9));
    assert!(!save_botch.stays_conscious && save_botch.falls_unconscious);
}

#[test]
fn test_status_conditions_and_acute_mental_trauma() {
    let nausea_dup = evaluate_physiological_status_conditions(&[
        PhysiologicalStatusCondition::NauseaSickened,
        PhysiologicalStatusCondition::NauseaSickened,
    ]);
    assert_eq!(nausea_dup.delta_int_added, 4);

    let paralyzed =
        evaluate_physiological_status_conditions(&[PhysiologicalStatusCondition::Paralyzed]);
    assert!(paralyzed.tempo_locked_zero && paralyzed.control_and_force_locked_zero);
    assert!(!paralyzed.awr_locked_zero);
    assert!(!paralyzed.to_condition_impact().is_unconscious);

    let unconscious = evaluate_physiological_status_conditions(&[
        PhysiologicalStatusCondition::UnconsciousComatose,
    ]);
    assert!(unconscious.awr_locked_zero);
    assert!(unconscious.to_condition_impact().is_unconscious);

    let t3 = evaluate_acute_mental_trauma(
        AcuteMentalTraumaTier::Tier3DissociationShellShock,
        false,
        10,
    );
    assert_eq!((t3.ana_foc_penalty, t3.dissociation_stat_penalty), (0, 6));
}

#[test]
fn test_movement_gaits_and_encumbrance_tiers_0_to_4() {
    let sp10 = compute_stride_and_sprint_velocity(10, 1.75);
    assert!((sp10.v_sprint_m_per_sec - 7.50).abs() < 1e-6);
    assert!((sp10.v_sprint_m_per_tick - 1.875).abs() < 1e-6);

    let sp18 = compute_stride_and_sprint_velocity(18, 1.75);
    assert!((sp18.v_sprint_m_per_sec - 10.30).abs() < 1e-6);

    let bound = evaluate_combat_movement_gait(CombatMovementGait::BoundingRunHustle, 10, 1.75, 11);
    assert_eq!(
        (
            bound.ranged_recoil_adder,
            bound.effective_ranged_control_cap,
            bound.exertion_marks_per_20_ticks
        ),
        (4, 5, 1)
    );

    let climb_ladder =
        evaluate_climbing_movement(true, LoadEncumbranceTier::Tier2TacticalFightingLoad, 2);
    assert_eq!(climb_ladder, (0.5, 2.0, false, 0));
    let climb_wall =
        evaluate_climbing_movement(false, LoadEncumbranceTier::Tier2TacticalFightingLoad, 2);
    assert_eq!(climb_wall, (0.15, 0.6, true, 6));

    let eva = evaluate_zero_g_maneuver(140.0, 70.0, false);
    assert!((eva.0 - 2.0).abs() < 1e-6 && eva.1);

    assert!((compute_base_comfort_load_kg(70.0, 10) - 12.0).abs() < 1e-6);
    assert!((compute_base_comfort_load_kg(85.0, 14) - 20.4).abs() < 1e-6);

    let enc_t2 = evaluate_encumbrance_load(
        70.0,
        10,
        &[CarriedLoadItem {
            mass_kg: 24.0,
            carry_method: CarryErgonomicMethod::BalancedWebbingOrRucksack,
        }],
        0.0,
    );
    assert_eq!(enc_t2.tier, LoadEncumbranceTier::Tier2TacticalFightingLoad);
    assert_eq!((enc_t2.tempo_cap_penalty, enc_t2.kin_penalty), (3, 2));
    assert!((enc_t2.action_time_factor_delta - 0.10).abs() < 1e-6);

    let enc_exo = evaluate_encumbrance_load(
        70.0,
        10,
        &[CarriedLoadItem {
            mass_kg: 24.0,
            carry_method: CarryErgonomicMethod::BalancedWebbingOrRucksack,
        }],
        20.0,
    );
    assert_eq!(enc_exo.tier, LoadEncumbranceTier::Tier0Unencumbered);
}

#[test]
fn test_environmental_survival_dehydration_starvation_thermal_sleep_vacuum() {
    let dehyd = evaluate_dehydration(
        HydrationEnvironment::HotAridDesertOrJungleTactical,
        4.0,
        0.0,
    );
    assert_eq!(dehyd.stage, DehydrationStage::Stage2Parched);
    assert_eq!((dehyd.delta_int, dehyd.fatigue_level_floor), (4, 2));
    assert_eq!(enforce_fatigue_floor(0, &[dehyd.fatigue_level_floor]), 2);

    let dehyd_sweat = evaluate_dehydration(HydrationEnvironment::TemperateLowActivity, 2.25, 2.0);
    assert_eq!(dehyd_sweat.stage, DehydrationStage::Stage2Parched);

    let starv = evaluate_starvation_and_caloric_deficit(6 * 24, 18_000, 10, 10, 10);
    assert_eq!(
        (starv.catabolic_intervals, starv.for_fit_hlt_erosion),
        (2, 2)
    );
    assert!((starv.mass_lost_kg - 3.0).abs() < 1e-6);

    let cramps =
        evaluate_thermal_stress(HypothermiaStage::None, HyperthermiaStage::Stage1HeatCramps);
    assert_eq!(cramps.tempo_and_force_cap_penalty, 2);
    assert_eq!(
        cramps.to_condition_impact().triad_cap_penalty,
        Triad::new(2, 0, 2)
    );

    assert_eq!(
        evaluate_frostbite_exposure(-15.0, true, false),
        Some(StructuralWoundTier::Tier1Superficial)
    );
    assert_eq!(
        evaluate_frostbite_exposure(-30.0, true, false),
        Some(StructuralWoundTier::Tier3SevereFracture)
    );

    let sleep = evaluate_sleep_deprivation(72);
    assert_eq!((sleep.delta_int, sleep.mental_sensory_ooda_penalty), (8, 6));
    assert!(sleep.requires_microsleep_dis_check);

    let alt_2600 = evaluate_altitude_and_vacuum_hypoxia(2600, false, false, 0, 10);
    assert_eq!(alt_2600.fit_penalty, 2);

    let vac = evaluate_altitude_and_vacuum_hypoxia(0, false, true, 68, 10);
    assert_eq!(
        (
            vac.vacuum_useful_consciousness_ticks,
            vac.vacuum_hlt_erosion
        ),
        (48, 2)
    );
    assert!(vac.vacuum_unconscious);
}

#[test]
fn test_combat_techniques_catalog_and_tcf_orthogonality() {
    assert_eq!(canonical_combat_technique_catalog().len(), 30);
    assert_eq!(
        combat_techniques_for_category(CombatMoveCategory::Control).len(),
        7
    );
    assert_eq!(
        combat_techniques_for_category(CombatMoveCategory::Strike).len(),
        7
    );
    assert_eq!(
        combat_techniques_for_category(CombatMoveCategory::Intercept).len(),
        2
    );
    assert_eq!(
        combat_techniques_for_category(CombatMoveCategory::Maneuver).len(),
        5
    );
    assert_eq!(
        combat_techniques_for_category(CombatMoveCategory::RangedShot).len(),
        5
    );
    assert_eq!(
        combat_techniques_for_category(CombatMoveCategory::RangedManeuver).len(),
        4
    );

    // Armed Melee allows Disarm, Push, Trap (Control), Thrust, Swing (Strike), Block, Parry (Intercept), and all 5 Maneuvers
    for allowed_armed in [
        CombatTechniqueId::Disarm,
        CombatTechniqueId::Push,
        CombatTechniqueId::Trap,
        CombatTechniqueId::Thrust,
        CombatTechniqueId::Swing,
        CombatTechniqueId::Block,
        CombatTechniqueId::Parry,
        CombatTechniqueId::Feint,
        CombatTechniqueId::Dodge,
        CombatTechniqueId::Escape,
        CombatTechniqueId::Probe,
        CombatTechniqueId::Root,
    ] {
        assert!(
            is_technique_allowed_in_domain(allowed_armed, CombatDomain::ArmedMelee),
            "{allowed_armed:?} should be allowed in ArmedMelee"
        );
    }
    // Grab, Takedown, Throw, Lock, and unarmed strikes are disallowed in ArmedMelee
    for unarmed_only in [
        CombatTechniqueId::Grab,
        CombatTechniqueId::Takedown,
        CombatTechniqueId::Throw,
        CombatTechniqueId::Lock,
        CombatTechniqueId::Punch,
        CombatTechniqueId::Elbow,
        CombatTechniqueId::Knee,
        CombatTechniqueId::Kick,
        CombatTechniqueId::HeadButt,
    ] {
        assert!(
            !is_technique_allowed_in_domain(unarmed_only, CombatDomain::ArmedMelee),
            "{unarmed_only:?} should NOT be allowed in ArmedMelee"
        );
        assert!(is_technique_allowed_in_domain(
            unarmed_only,
            CombatDomain::UnarmedMartialArts
        ));
    }

    // Orthogonality: Same Technique (`Punch`), different TCF Triad Allocations -> Jab Probe vs. Knockout Haymaker
    let env = compute_tactical_pool(&TacticalPoolParams {
        chars: CharacterCharacteristics {
            spd: 13,
            kin: 13,
            for_stat: 13,
            ..Default::default()
        },
        skill_c: 13,
        skill_n: 4,
        ..Default::default()
    });

    let jab_punch = evaluate_technique_with_triad(&TechniqueTriadEvalParams {
        technique: CombatTechniqueId::Punch,
        domain: CombatDomain::UnarmedMartialArts,
        envelope: env.clone(),
        alloc: Triad::new(11, 10, 2),
        skill_level_n: 4,
        ..Default::default()
    });
    let haymaker_punch = evaluate_technique_with_triad(&TechniqueTriadEvalParams {
        technique: CombatTechniqueId::Punch,
        domain: CombatDomain::UnarmedMartialArts,
        envelope: env.clone(),
        alloc: Triad::new(3, 3, 13),
        skill_level_n: 4,
        ..Default::default()
    });
    assert!(jab_punch.valid && haymaker_punch.valid);
    assert!(jab_punch.effective_tc < haymaker_punch.effective_tc);
    assert!(jab_punch.effective_guard > haymaker_punch.effective_guard);
    assert!(haymaker_punch.strike_impact > jab_punch.strike_impact * 4.0);

    // Unarmed Strike Progression: HeadButt (TC 5), Elbow (TC 6, +1 Guard), Punch (TC 7), Knee (TC 7, 1.25x POW), Kick (TC 9, 1.35x POW, -1 Guard, -2 Root)
    let balanced = Triad::new(7, 7, 7);
    let elbow = evaluate_technique_with_triad(&TechniqueTriadEvalParams {
        technique: CombatTechniqueId::Elbow,
        domain: CombatDomain::UnarmedMartialArts,
        envelope: env.clone(),
        alloc: balanced,
        skill_level_n: 4,
        ..Default::default()
    });
    let kick = evaluate_technique_with_triad(&TechniqueTriadEvalParams {
        technique: CombatTechniqueId::Kick,
        domain: CombatDomain::UnarmedMartialArts,
        envelope: env.clone(),
        alloc: balanced,
        skill_level_n: 4,
        ..Default::default()
    });
    assert!(elbow.effective_tc < kick.effective_tc);
    assert!(elbow.effective_guard > kick.effective_guard);
    assert!(elbow.effective_root > kick.effective_root);
    assert!(kick.strike_impact > elbow.strike_impact);

    // Armed Melee: Thrust vs. Swing with Tactical Blade (Base TC 9, Guard +1, POW 1.15x)
    let thrust = evaluate_technique_with_triad(&TechniqueTriadEvalParams {
        technique: CombatTechniqueId::Thrust,
        domain: CombatDomain::ArmedMelee,
        envelope: env.clone(),
        alloc: balanced,
        skill_level_n: 4,
        armed_weapon_base_tc: 9,
        armed_weapon_pow_mult: 1.15,
        weapon_guard_bonus: 1,
        ..Default::default()
    });
    let swing = evaluate_technique_with_triad(&TechniqueTriadEvalParams {
        technique: CombatTechniqueId::Swing,
        domain: CombatDomain::ArmedMelee,
        envelope: env.clone(),
        alloc: balanced,
        skill_level_n: 4,
        armed_weapon_base_tc: 9,
        armed_weapon_pow_mult: 1.15,
        weapon_guard_bonus: 1,
        ..Default::default()
    });
    assert!(thrust.valid && swing.valid);
    assert!(thrust.effective_tc < swing.effective_tc);
    assert_eq!(thrust.effective_guard, swing.effective_guard + 2);
    assert_eq!(thrust.effective_pen_mod, 1);
    assert!(swing.strike_impact > thrust.strike_impact);

    // Intercept: Block vs. Parry against a missed low-Control Haymaker
    let parry_def = evaluate_technique_with_triad(&TechniqueTriadEvalParams {
        technique: CombatTechniqueId::Parry,
        domain: CombatDomain::UnarmedMartialArts,
        envelope: env.clone(),
        alloc: Triad::new(8, 10, 4),
        skill_level_n: 4,
        ..Default::default()
    });
    let ex_parry = resolve_technique_exchange(
        &haymaker_punch,
        &parry_def,
        &TechniqueExchangeContext {
            attacker_net_successes: 0,
            ..Default::default()
        },
    );
    assert!(ex_parry.defender_seizes_initiative);
    assert_eq!(
        ex_parry.attacker_recovery_tick_penalty,
        (parry_def.effective_guard - haymaker_punch.effective_guard) * 2
    );

    // Ranged: SnapShot vs. DeliberateAimedShot vs. Braced Shot
    let snap = evaluate_technique_with_triad(&TechniqueTriadEvalParams {
        technique: CombatTechniqueId::SnapShot,
        domain: CombatDomain::Ranged,
        envelope: env.clone(),
        alloc: Triad::new(10, 8, 4),
        skill_level_n: 4,
        weapon_recoil_r: 5,
        weapon_acc: 3,
        weapon_is_braced: false,
        ..Default::default()
    });
    let aimed_braced = evaluate_technique_with_triad(&TechniqueTriadEvalParams {
        technique: CombatTechniqueId::DeliberateAimedShot,
        domain: CombatDomain::Ranged,
        envelope: env,
        alloc: Triad::new(5, 12, 4),
        skill_level_n: 4,
        weapon_recoil_r: 5,
        weapon_acc: 3,
        weapon_is_braced: true,
        ..Default::default()
    });
    assert!(snap.effective_tc < aimed_braced.effective_tc);
    // SnapShot ignores positive Acc (+3 -> 0) and suffers +1 unbraced recoil (R=5 vs F=4) + +2 SnapShot mod = +3 delta_ext
    assert_eq!(snap.ranged_effective_acc_bonus, 0);
    assert_eq!(snap.ranged_unbraced_dispersion_delta_ext, 1);
    assert_eq!(snap.technique_delta_ext, 3);
    // DeliberateAimedShot while braced slashes R_eff from 5+1=6 down to 3 (so F=4 fully braces it!), applies full Acc=+3, -2 Aim mod, and -1 Brace mod = -6 delta_ext!
    assert_eq!(aimed_braced.ranged_effective_recoil_r, 3);
    assert_eq!(aimed_braced.ranged_unbraced_dispersion_delta_ext, 0);
    assert_eq!(aimed_braced.ranged_effective_acc_bonus, 3);
    assert_eq!(aimed_braced.technique_delta_ext, -6);
}
