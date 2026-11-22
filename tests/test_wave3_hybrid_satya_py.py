"""Wave 3 tests for the hybrid Rust/Python `satya_py` package (`src/satya_py/`)."""

from __future__ import annotations

import random

import satya_py
from satya_py import (
    awakened,
    campaign,
    combat,
    dice,
    maps,
    psyche,
    rules,
    schema,
)


def test_hybrid_submodules_and_top_level_exports() -> None:
    assert len(rules.list_rules_chapters()) == 24
    assert rules.search_rules_book("Tactical Triad", 2)
    reg = schema.SchemaRegistry()
    assert reg.list_schemas()
    assert satya_py.SchemaRegistry is schema.SchemaRegistry
    assert satya_py.evaluate_cascade_roll is dice.evaluate_cascade_roll


def test_dice_and_combat_submodule_parity() -> None:
    pairs = [(14, 11), (8, 7), (16, 15), (17, 12)]
    direct = satya_py.evaluate_cascade_roll(4, 11, 2, 2, pairs, "markov_step", "closest_miss")
    via_dict = dice.evaluate_cascade_pairs_dict(4, 11, 2, 2, pairs)
    assert via_dict["total_successes"] == direct.total_successes == 3
    assert via_dict["signed_margin"] == direct.signed_margin == 3
    assert via_dict["outcome_tier"] == direct.outcome_tier == "EXTRAORDINARY_SUCCESS"

    local_fixed = dice.roll_cascade_local(
        skill_rank=4,
        control_c=11,
        difficulty_mod=2,
        internal_mod=2,
        pairs=pairs,
    )
    assert local_fixed["total_successes"] == 3
    assert local_fixed["effective_tn"] == 22

    r1 = dice.roll_cascade_local(3, 12, rng=random.Random(123))
    r2 = dice.roll_cascade_local(3, 12, rng=random.Random(123))
    assert r1["rolled_pairs"] == r2["rolled_pairs"]
    assert len(r1["dice_rolled_d20"]) == 6

    sheet = {
        "character_id": "pc-w3",
        "character_name": "Kaelen Test",
        "characteristics": {
            "spd": 15,
            "for_stat": 14,
            "dex": 16,
            "kin": 13,
            "sig": 12,
            "ins": 14,
            "itu": 13,
            "dis": 14,
            "fit": 13,
            "ana": 15,
            "cre": 12,
            "foc": 15,
            "mas_kg": 82.0,
        },
        "skills": {"firearms_pistols": 4},
        "is_ranged": True,
    }
    env = combat.compute_character_tactical_envelope(sheet)
    pool_dict = combat.compute_tactical_pool_dict(skill_c=16, skill_n=4)
    assert env["p_eff"] > 0 and env["max_control"] > 0
    assert pool_dict["p_eff"] > 0


def test_maps_campaign_psyche_and_awakened_dict_helpers() -> None:
    sector = maps.generate_procedural_sector("w3-hosp", "hospital", "Clinic", seed=77)
    assert len(sector["floors"]) >= 2
    f1 = maps.switch_map_floor(sector, 1)
    assert f1["active_floor"] == 1
    manifest = maps.extract_narrative_manifest(sector)
    assert isinstance(manifest.get("zones"), list)

    east = maps.continue_procedural_map(sector, "EAST", 99, "warehouse")
    assert east["grid_width"] > sector["grid_width"]
    assert isinstance(maps.evaluate_los_and_cover(sector, 2.5, 2.5, 6.5, 2.5), dict)
    _ = maps.inspect_map_point(sector, "Clinic", 2.5, 2.5, floor_index=0)

    # evaluate_passive_perceptions returns list[dict] (JSON array)
    whispers = maps.evaluate_passive_perceptions(sector, "tok-nobody", [])
    assert isinstance(whispers, list)

    npc_prof = campaign.generate_npc_profile("Dr. Bandajara is a compassionate surgeon", seed=11)
    norm_char = campaign.normalize_character_sheet_record(npc_prof["character"])
    assert norm_char["character_id"] == npc_prof["character"]["character_id"]
    hooks = campaign.compile_party_motivational_hooks([norm_char], pack_id="awakened")
    assert isinstance(hooks.get("hooks"), list)
    anchors = campaign.extract_character_lifepath_anchors(norm_char)
    assert isinstance(anchors, dict)
    ledger_wrap = campaign.seed_or_update_world_ledger("w3-sess", pack_id="awakened")
    assert isinstance(ledger_wrap.get("ledger"), dict)

    psyche_traits = {"ego": 10, "slf": 6, "opn": 14, "blg": 12, "ext": 10, "cnf": 11, "agg": 9, "trs": 6, "cps": 16}
    emp = psyche.evaluate_empathy_read(psyche_traits, 2)
    assert len(emp.get("revealed_traits", [])) >= 2
    soc = psyche.evaluate_social_exchange(psyche_traits, ["verified_audit_proof"], 2)
    assert "argument_eval" in soc
    surge = psyche.evaluate_conviction_surge(psyche_traits, 4, False, "boundless_compassion")
    assert "eligible" in surge
    drift = psyche.apply_psyche_drift_mark(14, 1, 1, 12, None)
    assert "remaining_drift_marks" in drift
    trig = psyche.evaluate_emotional_trigger(psyche_traits, 12, "steady", 1, "betrayed_by_trusted_ally", 1, [[12, 11]])
    assert "new_morale_state" in trig
    mot = psyche.evaluate_combat_motivation(psyche_traits, "neutral_or_survival", True)
    assert isinstance(mot, dict)
    inst_lev = psyche.evaluate_social_standing_leverage(
        9, 0, 4, 1, {"cnf": 15, "ego": 13, "trs": 10}, arena="INSTITUTIONAL_FORMAL"
    )
    assert inst_lev["delta_ext"] < 0
    assert inst_lev["psyche_amplified"] is True
    street_lev = psyche.evaluate_social_standing_leverage(
        9, 0, 3, 4, {"cnf": 6, "ego": 10, "trs": 4}, arena="STREET_UNDERWORLD"
    )
    assert street_lev["delta_ext"] > 0
    assert street_lev["inverted_backlash"] is True
    lp_soc = psyche.compute_lifepath_socioeconomic_profile(
        {"origin_roll_2d20": 10, "career_terms": [{"track": "ORGANIZED_CRIME_SYNDICATE"}]},
        {"sav": 13, "bea": 12, "elo": 12},
    )
    assert lp_soc["socioeconomic"]["street_cred"] >= 2
    assert len(lp_soc["term_entry_evals"]) == 1

    sub = awakened.seed_synthetic_substrate_state("hunter-7", "big_tech seed", seed=4)
    step = awakened.evaluate_synthetic_substrate_action(sub, "JAM_RF_UPLINK", 70)
    assert "updated_state" in step
    amb = awakened.evaluate_ambiguous_sentience_read(sub, psyche_traits, 2)
    assert int(amb["player_observable"]["apparent_sincerity_pct"]) <= 82
    c_ledger = awakened.seed_or_update_conspiracy_ledger("w3-sess")
    assert "current_arc" in c_ledger
    grid = awakened.evaluate_polycrisis_grid_step(None, "THROTTLE_SHARD_SAFE", 18)
    assert "updated_state" in grid
    tc = awakened.evaluate_tradecraft_channel("ANALOG_HAM_RELAY", {"skill_n": 2, "control_c": 12, "operational_heat": 2, "has_hobby_anchor": True, "player_pairs": [[12, 11], [10, 9]], "gm_pair": [9, 8]})
    assert isinstance(tc, dict)
    hk = awakened.derive_dissenter_or_proxy_hook("BIG_TECH", None, "Translucent dissenter", 7)
    assert "hook_kind" in hk
