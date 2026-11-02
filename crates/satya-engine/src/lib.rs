//! # `satya-engine`
//!
//! Satya TTRPG game engine, exact combinatorial & dynamic-programming dice probability distributions,
//! Two-List Cascading ("Watershed" / "Recoil") task resolution, and odds calibration sweeps.
//!
//! ## Module Structure
//! - [`dice_probabilities`]: Exact PMF combinatorial/DP engines, summary moments (`DistributionStats`),
//!   and Four-Factor Two-List Cascade game-engine task evaluators (`CascadeTaskParams`, `CascadeRollOutcome`,
//!   `evaluate_cascade_roll`, `evaluate_satya_cascade_task`).
//! - [`summarize_cascade_odds`]: Odds summary table generator and `K = 10/20/100` calibration sweeps.

#![deny(missing_docs)]

pub mod dice_probabilities;
pub mod summarize_cascade_odds;

pub use dice_probabilities::*;
pub use summarize_cascade_odds::*;

#[cfg(test)]
mod tests {
    use super::*;
    use satya_schema::{
        OutcomeTier, Participant, SchemaRegistry, SessionMode, SessionRole, subjects,
    };

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

        let c_vs_n_matrix = compare_control_vs_skill_k20(
            6,
            6,
            CarryMode::MarkovStep,
            TnFormulaK20::Formula31MinusC,
        );
        assert!(c_vs_n_matrix.contains("70.10%"));
        assert!(c_vs_n_matrix.contains("82.69%"));
        assert!(c_vs_n_matrix.contains("92.37%"));
    }
}
