"""Unit tests verifying exact probability distributions for Satya TTRPG dice mechanics."""

from __future__ import annotations

import math
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))

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


def test_rules_book_builder_and_tactical_pool_bridge() -> None:
    """Verify static HTML Rules Book generation, multi-chapter search, and Tactical Pool evaluation."""
    from satya_ml.ai_gm.tools import query_satya_rules
    from satya_ml.build_rules_book import (
        BOOK_CHAPTER_SPECS,
        build_static_rules_book,
        compute_tactical_pool_python,
        evaluate_cascade_pairs_python,
        evaluate_tactical_triad_python,
        search_rules_book_python,
    )

    assert len(BOOK_CHAPTER_SPECS) == 14
    summary = build_static_rules_book()
    assert summary["chapter_count"] == 14
    assert summary["using_satya_py"] is True
    assert len(summary["generated_files"]) == 16  # 14 chapters + index.html + catalog.json

    hits_melee = search_rules_book_python("Tactical Triad Tempo Control Force", max_results=3)
    assert hits_melee
    assert any(h["chapter_id"] in {"melee_combat", "combat"} for h in hits_melee)

    hits_health = search_rules_book_python("hemorrhaging broken bones blinded catatonia", max_results=3)
    assert hits_health
    assert hits_health[0]["chapter_id"] == "health_and_trauma"

    hits_env = search_rules_book_python("hypothermia dehydration encumbrance m/tick", max_results=3)
    assert hits_env
    assert hits_env[0]["chapter_id"] == "movement_and_environment"

    hits_ticks = search_rules_book_python("240 ticks in a minute 4 ticks per second", max_results=3)
    assert hits_ticks
    assert hits_ticks[0]["chapter_id"] == "action_economy"

    gm_excerpt = query_satya_rules.invoke({"query": "lockpicking Complementary KNO PHY synergy"})
    assert "tasks.md" in gm_excerpt or "Complementary" in gm_excerpt

    trace = evaluate_cascade_pairs_python(
        skill_level=4,
        control_c=11,
        internal_mod=2,
        external_mod=2,
        pairs=[(14, 11), (8, 7), (16, 15), (17, 12)],
    )
    assert trace["total_successes"] == 3
    assert trace["outcome_tier"] == "EXTRAORDINARY_SUCCESS"

    env = compute_tactical_pool_python(skill_c=14, skill_n=4, morale_state="steady")
    assert env["p_eff"] > 0
    triad = evaluate_tactical_triad_python(env, base_weapon_tc=8, tempo=8, control=8, force=6)
    assert triad["valid"] is True
    assert triad["effective_tc"] >= 3



def test_rules_book_interactive_calculators_and_static_builder(tmp_path) -> None:
    """Verify the Tactical Pool, Melee Triad, Lifepath, Social Dissonance, and Static HTML Rules Book generator."""
    from satya_ml.ai_gm.tools import query_satya_rules
    from satya_ml.build_rules_book import build_rules_book
    from satya_ml.dice_probabilities import (
        compute_lifepath_skill_progression,
        compute_social_dissonance_check,
        compute_tactical_pool_envelope,
        evaluate_tactical_triad_contest,
    )

    pool = compute_tactical_pool_envelope(control_c=16, skill_level=5, num_successes=3)
    assert pool["base_envelope"] >= 8
    assert pool["effective_pool"] <= pool["base_envelope"]

    triad = evaluate_tactical_triad_contest(
        attacker_tempo=4,
        attacker_control=5,
        attacker_force=4,
        defender_tempo=2,
        defender_control=2,
        defender_force=2,
    )
    assert triad["seizes_initiative"] is True
    assert triad["guard_broken"] is True
    assert triad["penetrating_force"] > 0

    lifepath = compute_lifepath_skill_progression(hours_studied=1600, instruction_quality=1.2)
    assert lifepath["skill_level_n"] >= 4
    assert lifepath["hours_to_next_level"] > 0

    social = compute_social_dissonance_check(
        argument_margin=4,
        target_conviction=10,
        evidence_tier=3,
    )
    assert social["net_dissonance_shift"] > 0
    assert "Concession" in social["social_outcome"] or "Shift" in social["social_outcome"]

    # Build static HTML book into tmp_path and into docs/book
    summary = build_rules_book(output_dir=tmp_path)
    assert summary["chapter_count"] == 14
    assert summary["using_satya_py"] is True
    assert summary["total_words"] > 10_000
    assert (tmp_path / "index.html").exists()
    assert (tmp_path / "design_philosophy.html").exists()
    assert (tmp_path / "tasks.html").exists()
    assert (tmp_path / "skills_and_trees.html").exists()
    assert (tmp_path / "lifepaths_and_economy.html").exists()
    assert (tmp_path / "psyche_and_social.html").exists()
    assert (tmp_path / "tactical_triad.html").exists()
    assert (tmp_path / "melee_combat.html").exists()
    assert (tmp_path / "ranged_combat_and_armor.html").exists()
    assert (tmp_path / "action_economy.html").exists()
    assert (tmp_path / "health_and_trauma.html").exists()
    assert (tmp_path / "movement_and_environment.html").exists()
    assert (tmp_path / "catalog.json").exists()

    # Multi-chapter AI GM rules search
    melee_res = query_satya_rules.invoke({"query": "Tactical Triad Tempo Control Force Kuzushi"})
    assert "melee_combat.md" in melee_res or "Tempo" in melee_res

    social_res = query_satya_rules.invoke({"query": "Cognitive Dissonance belief shift"})
    assert "social_interactions.md" in social_res or "Dissonance" in social_res

    health_res = query_satya_rules.invoke({"query": "hemorrhaging blood volume broken bones catatonia"})
    assert "health_and_trauma.md" in health_res or "Hemorrhaging" in health_res


def test_satya_py_contested_cascade_and_plotly_builders() -> None:
    """Verify that `satya_ml.dice_probabilities` delegates directly to the Rust `satya_py` PyO3 extension."""
    from satya_ml.dice_probabilities import (
        classify_contested_outcome,
        compute_contested_cascade_odds,
        plot_clt_sum_pools,
        plot_signed_margin_spectrum,
        using_satya_py_backend,
    )

    assert using_satya_py_backend() is True
    assert sum_dice_pool_pmf.__module__ == "satya_py"

    kind, net_m = classify_contested_outcome(2, 0, 2, 2, 0, 2, "hybrid_signed_margin")
    assert kind == "GRAZING_TIE_SUCCESS"
    assert net_m == 0

    tiered = compute_contested_cascade_odds(
        player_skill_n=3,
        player_control_c=10,
        hidden_external_difficulty=0,
        gm_mode="tiered_single_pair",
        gm_over_tier_step=4,
    )
    assert math.isclose(
        tiered["prob_total_win"] + tiered["prob_clean_failure"] + tiered["prob_botch_failure"],
        1.0,
        abs_tol=1e-9,
    )

    fig_clt = plot_clt_sum_pools()
    assert len(fig_clt.data) == 8
    fig_spec = plot_signed_margin_spectrum()
    assert len(fig_spec.data) == 6





