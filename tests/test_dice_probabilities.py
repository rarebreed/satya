"""Unit tests verifying exact probability distributions for Satya TTRPG dice mechanics."""

from __future__ import annotations

import math

from satya_ml.dice_probabilities import (
    center_distance_selection_pmf,
    compute_stats,
    count_successes_pmf,
    exploding_pool_keep_highest_pmf,
    median_gated_bimodal_pmf,
    multiplicative_pool_pmf,
    order_statistic_pmf,
    paired_list_cascade_pmfs,
    paired_list_signed_margin_pmfs,
    single_exploding_die_pmf,
    sum_dice_pool_pmf,
)


def test_clt_convergence_and_negative_dice() -> None:
    """Verify that adding or subtracting dice converges toward zero skewness and zero excess kurtosis."""
    pmf_1d6 = sum_dice_pool_pmf(1, 6)
    pmf_2d6 = sum_dice_pool_pmf(2, 6)
    pmf_4d6 = sum_dice_pool_pmf(4, 6)
    pmf_signed = sum_dice_pool_pmf(3, 6, neg_dice=2)

    s1 = compute_stats(pmf_1d6)
    s2 = compute_stats(pmf_2d6)
    s4 = compute_stats(pmf_4d6)
    s_signed = compute_stats(pmf_signed)

    assert math.isclose(s1.mean, 3.5)
    assert math.isclose(s2.mean, 7.0)
    assert math.isclose(s4.mean, 14.0)
    assert math.isclose(s_signed.mean, 3.5)

    # All symmetric around their mean
    assert abs(s1.skewness) < 1e-9
    assert abs(s2.skewness) < 1e-9
    assert abs(s4.skewness) < 1e-9
    assert abs(s_signed.skewness) < 1e-9

    # Excess kurtosis strictly approaches 0 as dice count grows
    assert s1.excess_kurtosis < s2.excess_kurtosis < s4.excess_kurtosis < 0.0
    assert abs(s_signed.excess_kurtosis) < abs(s4.excess_kurtosis)


def test_exploding_dice_skewness_and_gaps() -> None:
    """Verify exploding d6 has forbidden multiples of 6 and positive skewness."""
    pmf = single_exploding_die_pmf(6, explode_min=6, max_explosions=3)
    stats = compute_stats(pmf)

    assert math.isclose(sum(pmf.values()), 1.0)
    assert stats.skewness > 1.0
    # 6, 12, 18 cannot occur when 6 explodes
    assert 6 not in pmf
    assert 12 not in pmf
    assert 18 not in pmf

    gumbel_pmf = exploding_pool_keep_highest_pmf(4, 6, explode_min=6, max_explosions=3)
    gumbel_stats = compute_stats(gumbel_pmf)
    assert gumbel_stats.skewness > 0.5


def test_order_statistics_discrete_beta_skewness() -> None:
    """Verify Roll 5d20 Keep k-th die transitions from right-skewed to symmetric to left-skewed."""
    low_stats = compute_stats(order_statistic_pmf(5, 20, 1))
    med_stats = compute_stats(order_statistic_pmf(5, 20, 3))
    high_stats = compute_stats(order_statistic_pmf(5, 20, 5))

    assert low_stats.skewness > 0.8
    assert abs(med_stats.skewness) < 1e-9
    assert high_stats.skewness < -0.8
    assert math.isclose(low_stats.skewness, -high_stats.skewness, abs_tol=1e-9)


def test_center_distance_focus_vs_chaos() -> None:
    """Verify Focus concentrates at center while Chaos creates a U-shaped bimodal distribution."""
    focus_pmf = center_distance_selection_pmf(3, 20, mode="closest")
    chaos_pmf = center_distance_selection_pmf(3, 20, mode="furthest")

    # In Focus, 10 and 11 are most probable; 1 and 20 are least probable
    assert focus_pmf[10] > focus_pmf[1]
    assert math.isclose(focus_pmf[10], focus_pmf[11])

    # In Chaos (U-shape), 1 and 20 are most probable; 10 and 11 are least probable
    assert chaos_pmf[1] > chaos_pmf[5] > chaos_pmf[10]
    assert math.isclose(chaos_pmf[1], chaos_pmf[20])


def test_median_gated_bimodal() -> None:
    """Verify median-gated 3d20 produces two distinct peaks with a valley at the gate."""
    pmf = median_gated_bimodal_pmf(3, 20, gate_tn=11, roll_under=False)
    assert math.isclose(sum(pmf.values()), 1.0)
    # Valley around 10-11, peaks near low and high ends
    assert pmf[2] > pmf[10]
    assert pmf[19] > pmf[11]


def test_paired_list_watershed_cascade() -> None:
    """Verify the two-list cascading dice mechanic transforms bell curve into a bimodal U-curve."""
    # Delta = 0 -> standard Binomial bell curve peaking at 2-3 out of 5
    bell_succ, _ = paired_list_cascade_pmfs(
        num_pairs=5, sides=6, characteristic_c=7, delta_over=0, delta_under=0
    )
    assert bell_succ[2] > bell_succ[0]
    assert bell_succ[3] > bell_succ[5]

    # Delta = 4 -> watershed U-curve where streaks (0 and 5) beat middle split (2 and 3),
    # and Final Pair Y_N splits into two symmetric humps at 3 (7-4) and 11 (7+4) with a valley at 7
    u_succ, markov_final_y = paired_list_cascade_pmfs(
        num_pairs=5, sides=6, characteristic_c=7, delta_over=4, delta_under=4, mode="markov_step"
    )
    assert u_succ[0] > u_succ[2]
    assert u_succ[5] > u_succ[3]
    assert markov_final_y[3] > markov_final_y[7]
    assert markov_final_y[11] > markov_final_y[7]
    assert math.isclose(markov_final_y[3], markov_final_y[11])

    # Linear margin trap: final_y collapses to 10d6 - 28 (unimodal peak at 7),
    # while success count follows Levy's Arcsine U-curve!
    arc_succ, linear_final_y = paired_list_cascade_pmfs(
        num_pairs=5, sides=6, characteristic_c=7, mode="linear_margin"
    )
    assert arc_succ[0] > arc_succ[2]
    assert arc_succ[5] > arc_succ[3]
    assert linear_final_y[7] > linear_final_y[4]
    assert linear_final_y[7] > linear_final_y[10]


def test_multiplicative_and_satya_d20_pools() -> None:
    """Verify multiplicative pool log-normal skew and Satya d20 critical-1 pool."""
    prod_pmf = multiplicative_pool_pmf(3, 6)
    prod_stats = compute_stats(prod_pmf)
    assert prod_stats.skewness > 1.2

    satya_pmf = count_successes_pmf(3, 20, target_num=10, roll_under=True, crit_on_one_double=True)
    # Max possible successes on 3d20 when nat 1 = 2 successes is 6
    assert max(satya_pmf.keys()) == 6
    assert math.isclose(satya_pmf[6], (1 / 20) ** 3)


def test_inverted_feedback_recoil_and_attractor() -> None:
    """Verify rolling UNDER C to add (+delta) and OVER C to subtract (-delta) creates negative feedback."""
    ind_succ, _ = paired_list_cascade_pmfs(
        num_pairs=5, sides=6, characteristic_c=7, delta_over=0, delta_under=0
    )
    recoil_succ, _ = paired_list_cascade_pmfs(
        num_pairs=5,
        sides=6,
        characteristic_c=7,
        delta_over=3,
        delta_under=3,
        invert_feedback=True,
    )
    # Negative feedback suppresses streaks: variance is strictly smaller than independent Binomial
    ind_stats = compute_stats(ind_succ)
    recoil_stats = compute_stats(recoil_succ)
    assert recoil_stats.variance < ind_stats.variance
    assert recoil_succ[2] > ind_succ[2]
    assert recoil_succ[0] < ind_succ[0]
    assert recoil_succ[5] < ind_succ[5]

    # In cumulative mode with C=11, negative feedback pulls the mean of Y_N from 7 up toward 11
    _, neg_cum_y = paired_list_cascade_pmfs(
        num_pairs=5,
        sides=6,
        characteristic_c=11,
        delta_over=2,
        delta_under=2,
        mode="cumulative_snowball",
        invert_feedback=True,
    )
    y_stats = compute_stats(neg_cum_y)
    assert y_stats.mean > 10.0


def test_signed_margin_degrees_of_failure() -> None:
    """Verify the -N..+N signed margin spectrum and botch rules."""
    # Fresh character (delta_over=0) can NEVER breach 2K ceiling on ceiling_breach rule
    fresh = paired_list_signed_margin_pmfs(
        num_pairs=5,
        sides=10,
        characteristic_c=10,
        delta_over=0,
        delta_under=3,
        mode="cumulative_snowball",
        count_under_as_success=True,
        botch_rule="ceiling_breach",
    )
    assert math.isclose(fresh.prob_botch_failure, 0.0, abs_tol=1e-12)
    assert math.isclose(
        fresh.prob_any_success + fresh.prob_clean_failure + fresh.prob_botch_failure,
        1.0,
    )

    # Fatigued character (delta_over=4) has non-zero botch failure risk (-1..-4)
    fatigued = paired_list_signed_margin_pmfs(
        num_pairs=5,
        sides=10,
        characteristic_c=10,
        delta_over=4,
        delta_under=3,
        mode="cumulative_snowball",
        count_under_as_success=True,
        botch_rule="ceiling_breach",
    )
    assert fatigued.prob_botch_failure > 0.15
    assert fatigued.signed_margin_pmf.get(-1, 0.0) > 0.0
    assert fatigued.signed_margin_pmf.get(-2, 0.0) > 0.0
    assert math.isclose(
        fatigued.prob_any_success + fatigued.prob_clean_failure + fatigued.prob_botch_failure,
        1.0,
    )

    # Overrun rule (Y_n > C_eff + K) increases botch risk when C_eff < K
    overrun = paired_list_signed_margin_pmfs(
        num_pairs=5,
        sides=10,
        characteristic_c=7,
        delta_over=3,
        delta_under=2,
        mode="markov_step",
        count_under_as_success=True,
        botch_rule="overrun_k",
    )
    ceil_at_7 = paired_list_signed_margin_pmfs(
        num_pairs=5,
        sides=10,
        characteristic_c=7,
        delta_over=3,
        delta_under=2,
        mode="markov_step",
        count_under_as_success=True,
        botch_rule="ceiling_breach",
    )
    assert overrun.prob_botch_failure > ceil_at_7.prob_botch_failure

    # Closest-Miss rule (default): adding more dice pairs N strictly REDUCES botch failure risk!
    novice_n2 = paired_list_signed_margin_pmfs(
        num_pairs=2,
        sides=10,
        characteristic_c=8,
        delta_over=3,
        delta_under=1,
        botch_rule="closest_miss",
    )
    journeyman_n4 = paired_list_signed_margin_pmfs(
        num_pairs=4,
        sides=10,
        characteristic_c=8,
        delta_over=3,
        delta_under=2,
        botch_rule="closest_miss",
    )
    master_n6 = paired_list_signed_margin_pmfs(
        num_pairs=6,
        sides=10,
        characteristic_c=8,
        delta_over=3,
        delta_under=4,
        botch_rule="closest_miss",
    )
    assert novice_n2.prob_botch_failure > journeyman_n4.prob_botch_failure > master_n6.prob_botch_failure
    assert novice_n2.prob_any_success < journeyman_n4.prob_any_success < master_n6.prob_any_success
    # Even the worst possible bucket (-4, -5) is strictly higher for Novice than Master!
    assert novice_n2.signed_margin_pmf.get(-4, 0.0) > master_n6.signed_margin_pmf.get(-4, 0.0)



