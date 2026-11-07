//! Unit tests for `satya-engine` probability distributions, Two-List `2d20` Watershed Cascade,
//! Task Resolution gates/tools/clocks, Tactical Pool "Envelope & Fill", and Rules Book catalog/search.

use super::*;
use satya_schema::{OutcomeTier, Participant, SchemaRegistry, SessionMode, SessionRole, subjects};

fn assert_close(a: f64, b: f64, tol: f64) {
    assert!(
        (a - b).abs() <= tol,
        "expected {a} to be within {tol} of {b} (diff = {})",
        (a - b).abs()
    );
}

#[test]
fn test_clt_convergence_and_negative_dice() {
    let pmf_1d6 = sum_dice_pool_pmf(1, 6, 0);
    let pmf_2d6 = sum_dice_pool_pmf(2, 6, 0);
    let pmf_4d6 = sum_dice_pool_pmf(4, 6, 0);
    let pmf_signed = sum_dice_pool_pmf(3, 6, 2);

    let s1 = compute_stats(&pmf_1d6);
    let s2 = compute_stats(&pmf_2d6);
    let s4 = compute_stats(&pmf_4d6);
    let s_signed = compute_stats(&pmf_signed);

    assert_close(s1.mean, 3.5, 1e-9);
    assert_close(s2.mean, 7.0, 1e-9);
    assert_close(s4.mean, 14.0, 1e-9);
    assert_close(s_signed.mean, 3.5, 1e-9);

    assert!(s1.skewness.abs() < 1e-9);
    assert!(s2.skewness.abs() < 1e-9);
    assert!(s4.skewness.abs() < 1e-9);
    assert!(s_signed.skewness.abs() < 1e-9);

    assert!(s1.excess_kurtosis < s2.excess_kurtosis);
    assert!(s2.excess_kurtosis < s4.excess_kurtosis);
    assert!(s4.excess_kurtosis < 0.0);
    assert!(s_signed.excess_kurtosis.abs() < s4.excess_kurtosis.abs());
}

#[test]
fn test_exploding_dice_skewness_and_gaps() {
    let pmf = single_exploding_die_pmf(6, Some(6), 3);
    let stats = compute_stats(&pmf);

    let total: f64 = pmf.values().sum();
    assert_close(total, 1.0, 1e-9);
    assert!(stats.skewness > 1.0);
    assert!(!pmf.contains_key(&6));
    assert!(!pmf.contains_key(&12));
    assert!(!pmf.contains_key(&18));

    let gumbel_pmf = exploding_pool_keep_highest_pmf(4, 6, Some(6), 3);
    let gumbel_stats = compute_stats(&gumbel_pmf);
    assert!(gumbel_stats.skewness > 0.5);
}

#[test]
fn test_order_statistics_discrete_beta_skewness() {
    let low_stats = compute_stats(&order_statistic_pmf(5, 20, 1));
    let med_stats = compute_stats(&order_statistic_pmf(5, 20, 3));
    let high_stats = compute_stats(&order_statistic_pmf(5, 20, 5));

    assert!(low_stats.skewness > 0.8);
    assert!(med_stats.skewness.abs() < 1e-9);
    assert!(high_stats.skewness < -0.8);
    assert_close(low_stats.skewness, -high_stats.skewness, 1e-9);
}

#[test]
fn test_center_distance_focus_vs_chaos() {
    let focus_pmf = center_distance_selection_pmf(3, 20, CenterDistanceMode::Closest);
    let chaos_pmf = center_distance_selection_pmf(3, 20, CenterDistanceMode::Furthest);

    assert!(focus_pmf[&10] > focus_pmf[&1]);
    assert_close(focus_pmf[&10], focus_pmf[&11], 1e-12);

    assert!(chaos_pmf[&1] > chaos_pmf[&5]);
    assert!(chaos_pmf[&5] > chaos_pmf[&10]);
    assert_close(chaos_pmf[&1], chaos_pmf[&20], 1e-12);
}

#[test]
fn test_median_gated_bimodal() {
    let pmf = median_gated_bimodal_pmf(3, 20, 11, false);
    let total: f64 = pmf.values().sum();
    assert_close(total, 1.0, 1e-9);
    assert!(pmf[&2] > pmf[&10]);
    assert!(pmf[&19] > pmf[&11]);
}

#[test]
fn test_paired_list_watershed_cascade() {
    let (bell_succ, _) =
        paired_list_cascade_pmfs(5, 6, 7, 0, 0, CarryMode::MarkovStep, false, false);
    assert!(bell_succ[&2] > bell_succ[&0]);
    assert!(bell_succ[&3] > bell_succ[&5]);

    let (u_succ, markov_final_y) =
        paired_list_cascade_pmfs(5, 6, 7, 4, 4, CarryMode::MarkovStep, false, false);
    assert!(u_succ[&0] > u_succ[&2]);
    assert!(u_succ[&5] > u_succ[&3]);
    assert!(markov_final_y[&3] > markov_final_y[&7]);
    assert!(markov_final_y[&11] > markov_final_y[&7]);
    assert_close(markov_final_y[&3], markov_final_y[&11], 1e-12);

    let (arc_succ, linear_final_y) =
        paired_list_cascade_pmfs(5, 6, 7, 3, 3, CarryMode::LinearMargin, false, false);
    assert!(arc_succ[&0] > arc_succ[&2]);
    assert!(arc_succ[&5] > arc_succ[&3]);
    assert!(linear_final_y[&7] > linear_final_y[&4]);
    assert!(linear_final_y[&7] > linear_final_y[&10]);
}

#[test]
fn test_multiplicative_and_satya_d20_pools() {
    let prod_pmf = multiplicative_pool_pmf(3, 6);
    let prod_stats = compute_stats(&prod_pmf);
    assert!(prod_stats.skewness > 1.2);

    let satya_pmf = count_successes_pmf(3, 20, 10, true, true);
    assert_eq!(satya_pmf.keys().max().copied(), Some(6));
    assert_close(satya_pmf[&6], (1.0_f64 / 20.0).powi(3), 1e-12);
}

#[test]
fn test_inverted_feedback_recoil_and_attractor() {
    let (ind_succ, _) =
        paired_list_cascade_pmfs(5, 6, 7, 0, 0, CarryMode::MarkovStep, false, false);
    let (recoil_succ, _) =
        paired_list_cascade_pmfs(5, 6, 7, 3, 3, CarryMode::MarkovStep, false, true);

    let ind_stats = compute_stats(&ind_succ);
    let recoil_stats = compute_stats(&recoil_succ);
    assert!(recoil_stats.variance < ind_stats.variance);
    assert!(recoil_succ[&2] > ind_succ[&2]);
    assert!(recoil_succ[&0] < ind_succ[&0]);
    assert!(recoil_succ[&5] < ind_succ[&5]);

    let (_, neg_cum_y) =
        paired_list_cascade_pmfs(5, 6, 11, 2, 2, CarryMode::CumulativeSnowball, false, true);
    let y_stats = compute_stats(&neg_cum_y);
    assert!(y_stats.mean > 10.0);
}

#[test]
fn test_signed_margin_degrees_of_failure() {
    let fresh = paired_list_signed_margin_pmfs(
        5,
        10,
        10,
        0,
        3,
        CarryMode::CumulativeSnowball,
        true,
        false,
        BotchRule::CeilingBreach,
        MarginAggregation::PiecewiseZeroFloor,
    );
    assert_close(fresh.prob_botch_failure, 0.0, 1e-12);
    assert_close(
        fresh.prob_any_success + fresh.prob_clean_failure + fresh.prob_botch_failure,
        1.0,
        1e-9,
    );

    let fatigued = paired_list_signed_margin_pmfs(
        5,
        10,
        10,
        4,
        3,
        CarryMode::CumulativeSnowball,
        true,
        false,
        BotchRule::CeilingBreach,
        MarginAggregation::PiecewiseZeroFloor,
    );
    assert!(fatigued.prob_botch_failure > 0.15);
    assert!(fatigued.signed_margin_pmf.get(&-1).copied().unwrap_or(0.0) > 0.0);
    assert!(fatigued.signed_margin_pmf.get(&-2).copied().unwrap_or(0.0) > 0.0);
    assert_close(
        fatigued.prob_any_success + fatigued.prob_clean_failure + fatigued.prob_botch_failure,
        1.0,
        1e-9,
    );

    let overrun = paired_list_signed_margin_pmfs(
        5,
        10,
        7,
        3,
        2,
        CarryMode::MarkovStep,
        true,
        false,
        BotchRule::OverrunK,
        MarginAggregation::PiecewiseZeroFloor,
    );
    let ceil_at_7 = paired_list_signed_margin_pmfs(
        5,
        10,
        7,
        3,
        2,
        CarryMode::MarkovStep,
        true,
        false,
        BotchRule::CeilingBreach,
        MarginAggregation::PiecewiseZeroFloor,
    );
    assert!(overrun.prob_botch_failure > ceil_at_7.prob_botch_failure);

    let novice_n2 = paired_list_signed_margin_pmfs(
        2,
        10,
        8,
        3,
        1,
        CarryMode::MarkovStep,
        true,
        false,
        BotchRule::ClosestMiss,
        MarginAggregation::PiecewiseZeroFloor,
    );
    let journeyman_n4 = paired_list_signed_margin_pmfs(
        4,
        10,
        8,
        3,
        2,
        CarryMode::MarkovStep,
        true,
        false,
        BotchRule::ClosestMiss,
        MarginAggregation::PiecewiseZeroFloor,
    );
    let master_n6 = paired_list_signed_margin_pmfs(
        6,
        10,
        8,
        3,
        4,
        CarryMode::MarkovStep,
        true,
        false,
        BotchRule::ClosestMiss,
        MarginAggregation::PiecewiseZeroFloor,
    );
    assert!(novice_n2.prob_botch_failure > journeyman_n4.prob_botch_failure);
    assert!(journeyman_n4.prob_botch_failure > master_n6.prob_botch_failure);
    assert!(novice_n2.prob_any_success < journeyman_n4.prob_any_success);
    assert!(journeyman_n4.prob_any_success < master_n6.prob_any_success);
    assert!(
        novice_n2.signed_margin_pmf.get(&-4).copied().unwrap_or(0.0)
            > master_n6.signed_margin_pmf.get(&-4).copied().unwrap_or(0.0)
    );
}

#[test]
fn test_probability_md_k20_calibration_and_game_engine_evaluator() {
    let params = CascadeTaskParams::new_k20(3, 10, 0, 0);
    assert_eq!(params.tn_eff(), 21);
    assert_eq!(params.effective_delta_under(), 6);

    let odds = params.exact_odds();
    let stats = compute_stats(&odds.signed_margin_pmf);
    assert_close(odds.prob_any_success * 100.0, 70.10, 0.01);
    assert_close(odds.prob_botch_failure * 100.0, 12.96, 0.01);
    assert_close(odds.prob_success_with_complication * 100.0, 1.13, 0.01);
    assert_close(stats.mean, 1.27, 0.01);

    let registry = SchemaRegistry::default();
    let meta = registry
        .build_metadata(
            99,
            subjects::TTRPG_MECHANICS,
            1,
            "sector-4",
            SessionMode::Ttrpg,
        )
        .unwrap();
    let actor = Participant::human("u-sara", "sara_v", "Sara", SessionRole::Player);
    let (event, outcome) = evaluate_satya_cascade_task(
        meta,
        actor,
        vec![],
        "Airlock Bypass".to_string(),
        &params,
        &[(7, 8), (1, 2), (15, 14)],
        None,
        0,
        0.0,
    );
    assert_eq!(outcome.total_successes, 1);
    assert_eq!(outcome.raw_botches, 1);
    assert_eq!(outcome.signed_margin, 1);
    assert_eq!(
        outcome.outcome_kind,
        CascadeOutcomeKind::SuccessWithComplication
    );
    assert_eq!(event.outcome_tier, OutcomeTier::PartialSuccess);
    assert_eq!(event.dice_rolled_d20, vec![7, 8, 1, 2, 15, 14]);
}

#[test]
fn test_rules_book_catalog_search_and_tactical_pool_envelope() {
    let chapters = list_rules_chapters();
    assert_eq!(chapters.len(), 17);
    assert!(chapters.iter().any(|c| c.chapter_id == "design_philosophy"));
    assert!(chapters.iter().any(|c| c.chapter_id == "notation_glossary"));
    assert!(chapters.iter().any(|c| c.chapter_id == "tasks"));
    assert!(
        chapters
            .iter()
            .any(|c| c.chapter_id == "task_requirements_catalog")
    );
    assert!(chapters.iter().any(|c| c.chapter_id == "skills_and_trees"));
    assert!(
        chapters
            .iter()
            .any(|c| c.chapter_id == "skills_trees_tactical_and_social")
    );
    assert!(
        chapters
            .iter()
            .any(|c| c.chapter_id == "lifepaths_and_economy")
    );
    assert!(chapters.iter().any(|c| c.chapter_id == "psyche_and_social"));
    assert!(chapters.iter().any(|c| c.chapter_id == "tactical_triad"));
    assert!(chapters.iter().any(|c| c.chapter_id == "melee_combat"));
    assert!(
        chapters
            .iter()
            .any(|c| c.chapter_id == "firearms_and_ballistics")
    );
    assert!(
        chapters
            .iter()
            .any(|c| c.chapter_id == "ranged_combat_and_armor")
    );
    assert!(chapters.iter().any(|c| c.chapter_id == "action_economy"));
    assert!(chapters.iter().any(|c| c.chapter_id == "health_and_trauma"));
    assert!(
        chapters
            .iter()
            .any(|c| c.chapter_id == "movement_and_environment")
    );

    let tasks_doc = get_rules_chapter("tasks").expect("tasks chapter exists");
    assert!(tasks_doc.markdown.contains("Sarah"));
    assert!(tasks_doc.rendered_html.contains("satya-rules-article"));
    assert!(
        tasks_doc.rendered_html.contains("rules-math-inline"),
        "expected inline LaTeX math elements in rendered tasks chapter"
    );
    assert!(
        tasks_doc.rendered_html.contains("rules-math-block"),
        "expected display LaTeX math elements in rendered tasks chapter"
    );
    assert!(
        tasks_doc.rendered_html.contains("rules-mermaid"),
        "expected rendered Mermaid flowchart in tasks chapter"
    );
    assert!(
        tasks_doc.rendered_html.contains("mermaid-node"),
        "expected Mermaid flowchart nodes in tasks chapter"
    );
    assert!(
        !tasks_doc.rendered_html.contains(r"\text{TN}_P"),
        "expected raw LaTeX \\text{{TN}}_P to be rendered into HTML math"
    );
    assert!(
        !tasks_doc.rendered_html.contains("S1[\""),
        "expected raw Mermaid node syntax S1[\" to be parsed into flowchart cards"
    );
    for ch in &chapters {
        let doc = get_rules_chapter(&ch.chapter_id).expect("chapter exists");
        assert!(
            !doc.rendered_html.contains("&amp;amp;"),
            "chapter {} should not contain double-escaped &amp;amp;",
            ch.chapter_id
        );
        assert!(
            !doc.rendered_html.contains(r"\text{"),
            "chapter {} should not leak raw \\text{{...}} LaTeX commands",
            ch.chapter_id
        );
        assert!(
            !doc.rendered_html.contains(r"\frac{"),
            "chapter {} should not leak raw \\frac{{...}} LaTeX commands",
            ch.chapter_id
        );
    }
    // Verify currency strings in lifepaths_and_economy are preserved as literal $ amounts
    let economy_doc =
        get_rules_chapter("lifepaths_and_economy").expect("lifepaths_and_economy exists");
    assert!(economy_doc.rendered_html.contains("$500"));
    // Verify backward-compatible legacy chapter ID aliases
    assert!(get_rules_chapter("satya_d20_rules").is_some());
    assert!(get_rules_chapter("combat").is_some());
    assert!(get_rules_chapter("skill-design").is_some());

    let health_hits = search_rules_book("hemorrhaging broken bones blinded catatonia", 5);
    assert!(!health_hits.is_empty());
    assert_eq!(health_hits[0].chapter_id, "health_and_trauma");

    let hits = search_rules_book("Envelope & Fill Tempo Control Force", 5);
    assert!(!hits.is_empty());
    assert!(
        hits[0].chapter_id == "tactical_triad" || hits[0].chapter_id == "melee_combat",
        "expected tactical_triad or melee_combat chapter at top of search hits, got {}",
        hits[0].chapter_id
    );

    // Verify Elena's exact Envelope & Fill numbers from melee_combat.md
    let elena_params = TacticalPoolParams {
        chars: CharacterCharacteristics {
            spd: 13,
            for_stat: 10,
            dex: 12,
            kin: 13,
            sig: 12,
            ins: 12,
            itu: 13,
            ana: 12,
            cre: 10,
            foc: 12,
            dis: 13,
            fit: 12,
            mas_kg: 65.0,
            ..Default::default()
        },
        skill_c: 13,
        skill_n: 4,
        is_ranged: false,
        ..Default::default()
    };
    let elena_env = compute_tactical_pool(&elena_params);
    assert_eq!(elena_env.d_cog, 12);
    assert_eq!(elena_env.ooda_init, 12);
    assert_eq!(elena_env.combat_readiness_c, 12);
    assert_eq!(elena_env.p_base, 24);
    assert_eq!(elena_env.p_eff, 24);
    assert_eq!(elena_env.neutral_baseline_b, 8);
    assert_eq!(elena_env.caps.tempo, 13);
    assert_eq!(elena_env.caps.control, 13);
    assert_eq!(elena_env.caps.force, 10);

    let probe_eval = evaluate_tactical_triad(&elena_env, 8, Triad::new(11, 11, 2));
    assert!(probe_eval.valid);
    assert_eq!(probe_eval.effective_tc, 5);
    assert!(probe_eval.archetype_label.starts_with("TC"));
}

#[test]
fn test_contested_cascade_hybrid_and_live_rolls() {
    let contested_std = ContestedCascadeTaskParams::new_hybrid_environment(
        3,
        10,
        0,
        0,
        1,
        0,
        ContestedTieRule::HybridSignedMargin,
    );
    let odds = contested_std.exact_contested_odds();
    assert_close(
        odds.prob_total_win + odds.prob_clean_failure + odds.prob_botch_failure,
        1.0,
        1e-9,
    );
    assert_close(odds.prob_clean_or_extraordinary_win * 100.0, 59.39, 0.05);
    assert_close(odds.prob_grazing_tie_partial * 100.0, 10.50, 0.05);
    assert_close(odds.prob_lucky_break_partial * 100.0, 5.76, 0.05);

    // Live roll check: Grazing Tie (Player rolls 1 success, GM rolls 1 success)
    let graze_roll =
        contested_std.evaluate_contested_pairs(&[(12, 10), (5, 5), (5, 5)], &[(12, 10)]);
    assert_eq!(
        graze_roll.contested_kind,
        ContestedOutcomeKind::GrazingTieSuccess
    );
    assert_eq!(graze_roll.outcome_tier, OutcomeTier::PartialSuccess);

    // Live roll check: Lucky Break (Player rolls Clean Miss 0, GM botches with snake-eyes 2)
    let lucky_roll =
        contested_std.evaluate_contested_pairs(&[(10, 10), (12, 14), (10, 10)], &[(1, 1)]);
    assert_eq!(lucky_roll.player_outcome.total_successes, 0);
    assert_eq!(lucky_roll.player_outcome.signed_margin, 0);
    assert!(lucky_roll.gm_outcome.signed_margin < 0);
    // Verify 4-point Tiered 1-Pair GM Resistance Rule
    let tiered_std = ContestedCascadeTaskParams::new_tiered_environment(
        3,
        10,
        0,
        0,
        0,
        ContestedTieRule::HybridSignedMargin,
    );
    let tiered_odds = tiered_std.exact_contested_odds();
    assert_close(
        tiered_odds.prob_total_win
            + tiered_odds.prob_clean_failure
            + tiered_odds.prob_botch_failure,
        1.0,
        1e-9,
    );
    // Live roll check: GM rolls 28 against TN_GM=21 (over by 7 -> 2 successes at 4-pt step!)
    let tiered_roll =
        tiered_std.evaluate_contested_pairs(&[(12, 10), (12, 10), (5, 5)], &[(15, 13)]);
    assert_eq!(tiered_roll.player_outcome.total_successes, 2);
    assert_eq!(tiered_roll.gm_outcome.total_successes, 2);
    assert_eq!(
        tiered_roll.contested_kind,
        ContestedOutcomeKind::GrazingTieSuccess
    );
}

#[test]
fn test_task_gate_classes_and_tool_dependency_tiers() {
    // Class A untrained (N=0, no talent) -> N=1, gate_delta_ext = +4
    let res_a = resolve_task_gate_and_tools(
        TaskGateClass::ClassA,
        0,
        0,
        4,
        ToolDependency::None,
        ToolQualityTier::NoTools,
    );
    assert!(res_a.allowed);
    assert_eq!(res_a.effective_skill_n, 1);
    assert_eq!(res_a.gate_delta_ext, 4);
    assert_eq!(res_a.tool_delta_mod, 0);
    assert!(res_a.used_defaulting);

    // Class B untrained with sibling N_parent = 3 -> N = 2, gate_delta_ext = +4
    let res_b = resolve_task_gate_and_tools(
        TaskGateClass::ClassB,
        0,
        3,
        4,
        ToolDependency::Situational,
        ToolQualityTier::Improvised,
    );
    assert!(res_b.allowed);
    assert_eq!(res_b.effective_skill_n, 2);
    assert_eq!(res_b.gate_delta_ext, 4);
    assert_eq!(res_b.tool_delta_mod, 4);
    assert!(res_b.improvised_wear_on_floor_breach);

    // Class C [Locked] when N < min_skill_n
    let res_c_locked = resolve_task_gate_and_tools(
        TaskGateClass::ClassC { min_skill_n: 2 },
        1,
        3,
        4,
        ToolDependency::Mandatory,
        ToolQualityTier::StandardKit,
    );
    assert!(!res_c_locked.allowed);
    assert!(res_c_locked.lock_reason.is_some());

    // [Tool: Mandatory] + NoTools -> [Locked] even if trained
    let res_no_tools = resolve_task_gate_and_tools(
        TaskGateClass::ClassB,
        3,
        0,
        4,
        ToolDependency::Mandatory,
        ToolQualityTier::NoTools,
    );
    assert!(!res_no_tools.allowed);
}

#[test]
fn test_composite_and_supporting_characteristic_formulas() {
    // Sarah: DEX 13, TAC 12 -> floor((13 + 12)/2) = 12
    assert_eq!(compute_composite_characteristic(13, 12), 12);
    // Marcus: DEX 14, FOC 15 -> floor((14 + 15)/2) = 14
    assert_eq!(compute_composite_characteristic(14, 15), 14);
    // Supporting characteristic shortfall: Min DIS >= 13, character has DIS = 11 -> shortfall 2 -> +4 Delta_ext
    assert_eq!(
        compute_supporting_characteristic_shortfall_penalty(11, 13),
        4
    );
    assert_eq!(
        compute_supporting_characteristic_shortfall_penalty(13, 13),
        0
    );
}

#[test]
fn test_complementary_synergy_dual_skill_and_project_clocks() {
    // Elena's Intrusion Hack (§7.3): Dual-Skill Gate (Intrusion N=4, Cryptography N=2) -> N_eff = 3
    assert_eq!(compute_dual_skill_pool(4, 2), 3);
    assert_eq!(compute_dual_skill_pool(3, 2), 2);

    // Extraordinary setup (3 successes, margin +3) -> Delta_ext -4, delta_over = 8
    let syn = evaluate_complementary_setup_synergy(3, 3);
    assert_eq!(syn.delta_ext_mod, -4);
    assert_eq!(syn.delta_over_carry, 8);

    let base = ContestedCascadeTaskParams::new_tiered_environment(
        3,
        15,
        0,
        -2,
        6,
        ContestedTieRule::HybridSignedMargin,
    );
    let boosted = base.with_complementary_synergy(syn);
    assert_eq!(boosted.player.tn_eff(), 14);
    assert_eq!(boosted.gm.tn_eff(), 19); // 21 - (6 - 4) = 19
    assert_eq!(boosted.player.delta_over, 8);

    // Verify exact worked example 7.3 (Elena's Hack: (11,9)=20->succ, (6,5)+8=19->succ, (14,12)+8=34->succ; GM (10,8)=18 <= 19 -> 0)
    let out = boosted.evaluate_contested_pairs(&[(11, 9), (6, 5), (14, 12)], &[(10, 8)]);
    assert_eq!(out.player_outcome.total_successes, 3);
    assert_eq!(out.gm_outcome.total_successes, 0);
    assert_eq!(
        out.contested_kind,
        ContestedOutcomeKind::ExtraordinarySuccess
    );
    assert!((out.contested_kind.duration_multiplier(out.net_successes) - 0.70).abs() < 1e-6);

    // Extended project clock
    let mut clock = ExtendedProjectClock::new("Synthesize Antitoxin", 10);
    let step1 = clock.apply_contested_roll(&out);
    assert_eq!(step1.accumulated_successes, 3);
    assert!(!step1.is_completed);
}

#[test]
fn test_worked_examples_7_1_and_7_2_and_master_benchmarks() {
    // §7.1 Sarah's Lockpick
    let sarah = get_canonical_task_preset("sarah_lockpick").to_contested_params();
    assert_eq!(sarah.player.tn_eff(), 19);
    assert_eq!(sarah.gm.tn_eff(), 17);
    let sarah_roll = sarah.evaluate_contested_pairs(&[(11, 9), (7, 6), (12, 10)], &[(9, 9)]);
    assert_eq!(sarah_roll.player_outcome.total_successes, 2);
    assert_eq!(sarah_roll.gm_outcome.total_successes, 1);
    assert_eq!(
        sarah_roll.contested_kind,
        ContestedOutcomeKind::CleanSuccess
    );

    // §7.2 Marcus's Trauma Surgery (delta_under = 6 - 1 = 5; Floor breach on Pair 2: (3,3) - 5 = 1 < 2)
    let marcus = get_canonical_task_preset("marcus_surgery").to_contested_params();
    assert_eq!(marcus.player.tn_eff(), 15);
    assert_eq!(marcus.gm.tn_eff(), 17);
    let marcus_roll =
        marcus.evaluate_contested_pairs(&[(8, 6), (3, 3), (12, 11), (9, 8)], &[(10, 6)]);
    assert_eq!(marcus_roll.player_outcome.total_successes, 2);
    assert_eq!(marcus_roll.player_outcome.raw_botches, 1);
    assert_eq!(marcus_roll.gm_outcome.total_successes, 0);
    assert_eq!(
        marcus_roll.contested_kind,
        ContestedOutcomeKind::SuccessWithFloorBreach
    );

    assert_eq!(list_master_task_benchmarks().len(), 22);
}

#[test]
fn test_viktor_vs_elena_melee_walkthrough() {
    let elena_params = TacticalPoolParams {
        chars: CharacterCharacteristics {
            spd: 13,
            for_stat: 10,
            dex: 12,
            kin: 13,
            sig: 12,
            ins: 12,
            itu: 13,
            ana: 12,
            cre: 10,
            foc: 12,
            dis: 13,
            fit: 12,
            mas_kg: 65.0,
            ..Default::default()
        },
        skill_c: 13,
        skill_n: 4,
        is_ranged: false,
        ..Default::default()
    };
    let mut viktor_params = TacticalPoolParams {
        chars: CharacterCharacteristics {
            spd: 8,
            for_stat: 15,
            dex: 9,
            kin: 9,
            sig: 10,
            ins: 10,
            itu: 9,
            ana: 9,
            cre: 8,
            foc: 9,
            dis: 9,
            fit: 13,
            mas_kg: 102.0,
            ..Default::default()
        },
        skill_c: 11,
        skill_n: 2,
        is_ranged: false,
        ..Default::default()
    };

    let elena_env = compute_tactical_pool(&elena_params);
    let viktor_env = compute_tactical_pool(&viktor_params);
    assert_eq!((elena_env.p_base, elena_env.neutral_baseline_b), (24, 8));
    assert_eq!((viktor_env.p_base, viktor_env.neutral_baseline_b), (16, 5));
    assert_eq!(viktor_env.caps, Triad::new(8, 9, 15));
    assert!((viktor_env.power_pow - 12.0).abs() < 1e-6);

    let elena_ex1 = evaluate_tactical_triad(&elena_env, 8, Triad::new(11, 11, 2));
    assert!(elena_ex1.valid && elena_ex1.effective_tc == 5);

    viktor_params.wound_cap_penalties.tempo = 2;
    viktor_params.morale_state = MoraleState::Rage;
    let viktor_rage_env = compute_tactical_pool(&viktor_params);
    assert_eq!(
        (
            viktor_rage_env.p_eff,
            viktor_rage_env.tempo_cap,
            viktor_rage_env.min_force_required,
            viktor_rage_env.max_control_allowed
        ),
        (18, 6, 9, 4)
    );

    let viktor_haymaker = evaluate_tactical_triad(&viktor_rage_env, 10, Triad::new(3, 3, 12));
    assert!(viktor_haymaker.valid && viktor_haymaker.effective_tc == 12);

    let elena_ex2 = evaluate_tactical_triad(&elena_env, 8, Triad::new(6, 10, 8));
    assert!(elena_ex2.valid);
    let (interrupted, counter_land_tick) =
        check_melee_interrupt_before_impact(15, 6, elena_ex2.effective_tc - 2, true);
    assert!(interrupted && counter_land_tick < 15);

    viktor_params.morale_state = MoraleState::Steady;
    viktor_params.wound_shock_index = 2;
    viktor_params.fatigue_level = 1;
    let viktor_after = compute_tactical_pool(&viktor_params);
    assert_eq!(viktor_after.p_eff, 10);
}

#[test]
fn test_clinch_and_hip_throw_vs_knife_blitz() {
    let res = resolve_contested_melee_exchange(
        Triad::new(4, 12, 8),
        Triad::new(10, 2, 8),
        &MeleeExchangeContext {
            defender_guard_bonus: -1,
            defender_mmf: 12.0,
            attacker_reach: 1,
            defender_reach: 1,
            in_clinch: true,
            defender_has_keep_out_weapon: false,
            attacker_net_successes: 2,
            is_throw_or_sweep: true,
            redirect_opponent_blitz_in_clinch: true,
        },
    );
    assert!(res.kuzushi_throw_succeeded);
    assert!(res.leveraged_throw_force >= res.defender_effective_root);
    assert_eq!(res.prone_recovery_ticks, 6);
}

#[test]
fn test_mateo_vs_sgt_vance_firefight_suppression_walkthrough() {
    let mateo_ranged = evaluate_ranged_recoil_and_suppression(8, 8, false, true);
    assert!(mateo_ranged.recoil_fully_braced);
    assert_eq!(
        (
            mateo_ranged.unbraced_dispersion_delta_ext,
            mateo_ranged.suppression_morale_delta
        ),
        (0, 4)
    );

    // Vance scores 2 successes vs 1 GM success (net margin +1 -> holds Steady per §9.1 & §3.4)
    let vance_morale = evaluate_combat_morale_roll(
        14,
        0,
        0,
        mateo_ranged.suppression_morale_delta,
        true,
        MoraleState::Steady,
        &[(10, 9), (12, 13)],
        (8, 14),
    );
    assert_eq!(vance_morale.new_morale, MoraleState::Steady);

    // CleanSuccess with net_signed_margin >= +2 rallies +1 step toward High (§3.4)
    let vance_rally = evaluate_combat_morale_roll(
        14,
        0,
        0,
        mateo_ranged.suppression_morale_delta,
        true,
        MoraleState::Steady,
        &[(10, 9), (8, 8)],
        (8, 7),
    );
    assert_eq!(vance_rally.new_morale, MoraleState::High);

    let vance_ranged = evaluate_ranged_recoil_and_suppression(5, 5, false, false);
    assert!(vance_ranged.recoil_fully_braced);

    let mateo_profile = PsycheProfile {
        slf: 15,
        blg: 7,
        ..Default::default()
    };
    let mateo_mot = evaluate_combat_motivation_modifier(
        &mateo_profile,
        CombatMotivationKind::MercenaryTransactionalUnderFire,
    );
    assert_eq!(mateo_mot.delta_mot, -2);

    let mateo_morale_roll = evaluate_combat_morale_roll(
        9,
        mateo_mot.delta_mot,
        0,
        4,
        false,
        MoraleState::Steady,
        &[(4, 4)],
        (10, 10),
    );
    assert_eq!(mateo_morale_roll.new_morale, MoraleState::Broken);

    let mateo_broken_env = compute_tactical_pool(&TacticalPoolParams {
        chars: CharacterCharacteristics {
            spd: 12,
            for_stat: 12,
            dex: 8,
            sig: 8,
            ins: 12,
            itu: 12,
            ana: 12,
            foc: 12,
            ..Default::default()
        },
        skill_c: 12,
        skill_n: 2,
        is_ranged: true,
        wound_shock_index: 2,
        wound_cap_penalties: Triad::new(0, 5, 5),
        morale_state: MoraleState::Broken,
        ..Default::default()
    });
    assert_eq!(
        (
            mateo_broken_env.p_base,
            mateo_broken_env.p_eff,
            mateo_broken_env.max_force_allowed,
            mateo_broken_env.control_cap
        ),
        (18, 10, 0, 3)
    );

    let scramble = evaluate_tactical_triad(&mateo_broken_env, 8, Triad::new(7, 3, 0));
    assert!(scramble.valid);
}

#[test]
fn test_sniper_overwatch_and_grenade_dive_walkthrough() {
    let elena_env = compute_tactical_pool(&TacticalPoolParams {
        chars: CharacterCharacteristics {
            spd: 13,
            for_stat: 10,
            dex: 12,
            kin: 13,
            sig: 12,
            ins: 12,
            itu: 13,
            ana: 12,
            cre: 10,
            foc: 12,
            dis: 13,
            fit: 12,
            mas_kg: 65.0,
            ..Default::default()
        },
        skill_c: 13,
        skill_n: 4,
        is_ranged: false,
        ..Default::default()
    });
    let bound1 = evaluate_tactical_triad(&elena_env, 8, Triad::new(12, 12, 0));
    assert!(
        bound1.valid
            && bound1.archetype_label.starts_with("TC")
            && bound1.exertion_marks_gained == 1
    );

    let bound2 = evaluate_tactical_triad(&elena_env, 8, Triad::new(12, 2, 10));
    assert!(
        bound2.valid
            && bound2.archetype_label.starts_with("TF")
            && bound2.exertion_marks_gained == 2
    );
}

#[test]
fn test_armor_penetration_vs_babt_and_weapon_malfunctions() {
    assert_eq!(canonical_armor_catalog().len(), 7);
    let uhmwpe = ArmorProfileId::LightweightUhmwpePatrolCarrierNij3.spec();
    assert_eq!(
        (
            uhmwpe.pr_edged,
            uhmwpe.pr_icepick_flechette,
            uhmwpe.pr_ballistic,
            uhmwpe.blunt_trauma_dampening_btd,
            uhmwpe.cost_usd
        ),
        (5, 5, 6, 4, 1600)
    );

    let kevlar = ArmorProfileId::ConcealableSoftKevlarVestNij3a.spec();
    assert!((kevlar.weight_kg - 2.2).abs() < 1e-6);
    let stopped = resolve_armor_impact(kevlar, WeaponDamageType::Ballistic, 6, 2, 1, false);
    assert!(
        !stopped.penetrated
            && !stopped.causes_bleed
            && stopped.babt_trauma == 4
            && stopped.total_effective_trauma == 4
    );

    let overmatched = resolve_armor_impact(kevlar, WeaponDamageType::Ballistic, 9, 4, 2, false);
    assert!(
        overmatched.penetrated
            && overmatched.causes_bleed
            && overmatched.penetrating_trauma == 6
            && overmatched.armor_pr_degradation == 1
    );

    let plate_carrier = ArmorProfileId::TacticalPlateCarrierLevel4.spec();
    let ceramic_stop =
        resolve_armor_impact(plate_carrier, WeaponDamageType::Ballistic, 8, 6, 2, false);
    assert!(
        !ceramic_stop.penetrated
            && ceramic_stop.babt_trauma == 4
            && ceramic_stop.armor_pr_degradation == 1
    );

    let icepick =
        resolve_armor_impact(kevlar, WeaponDamageType::IcepickOrFlechette, 5, 1, 2, false);
    assert_eq!((icepick.active_armor_pr, icepick.effective_pen), (1, 2));
    assert!(icepick.penetrated);
}
