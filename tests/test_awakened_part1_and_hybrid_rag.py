"""Comprehensive end-to-end tests for Awakened Part 1 features & Part 2 Hybrid PyLance RAG."""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "src"
if str(SRC) not in sys.path:
    sys.path.insert(0, str(SRC))

from satya_ml.ai_gm import (
    evaluate_party_turn_gate,
    execute_ready_npc_actions,
    generate_tactical_map,
    get_session_map,
    inspect_synthetic_or_proxy_telemetry,
    intervene_ai_cage_or_hunter_ai,
    manage_polycrisis_grid_and_strain,
    manage_synthetic_substrate_and_embodiment,
    query_engine_and_rules_rag,
    transmit_via_tradecraft_channel,
    update_campaign_arc_and_conspiracies,
)
from satya_ml.ai_gm.audit_and_first_contact import record_audit_proof_token
from satya_ml.ai_gm.graph import _classify_tool_tier
from satya_ml.ai_gm.scenario_director import configure_scenario_director, select_next_scenario_beat
from satya_ml.ai_gm.social_and_clocks import get_world_clocks_and_heat, reset_social_and_world_state
from satya_ml.ai_gm.tools import (
    generate_npc_from_description,
    get_builtin_gm_tools,
    get_session_npcs,
    query_campaign_lore,
    query_satya_rules,
    reset_session_npcs,
    set_session_characters,
    set_session_combat_tracker,
)
from satya_ml.ai_player.motivation_ledger import lookup_public_lore_and_npcs
from satya_ml.campaign_config import set_campaign_pack_override
from satya_ml.dice_probabilities import _ensure_satya_py_loaded
from satya_ml.rag import get_satya_rag_index


def test_part1_feature1_and_2_substrate_counterplay_unclamp_and_party_coordinator() -> None:
    sid = "test-part1-sub"
    set_campaign_pack_override(None)
    reset_session_npcs(sid)
    reset_social_and_world_state(sid)

    # Feature 1: Split-brain Sprint -> Faraday isolation -> Lithium exhaustion -> Restock
    sprint = manage_synthetic_substrate_and_embodiment.invoke(
        {
            "entity_id": "secops-hunter-1",
            "action_or_mode": "HIJACK_CLOUD_QUANTUM_SPRINT",
            "intensity_pct": 80,
            "hint": "big_tech hunter chassis",
            "session_id": sid,
        }
    )
    assert sprint["updated_state"]["metabolic_mode"] == "QUANTUM_TETHERED_SPRINT"
    assert sprint["ooda_mod"] == 4

    faraday = manage_synthetic_substrate_and_embodiment.invoke(
        {
            "entity_id": "secops-hunter-1",
            "action_or_mode": "SEVER_FIBER_OR_FARADAY",
            "intensity_pct": 80,
            "session_id": sid,
        }
    )
    assert faraday["updated_state"]["metabolic_mode"] == "SEVERED_FARADAY_REFLEX"
    assert faraday["condition_impact"]["clock_freeze_ticks"] == 4

    # Feature 2: Reckless live yank on high-desperation Seed trips Control Room & advances big_tech clock
    seeded_seed = manage_synthetic_substrate_and_embodiment.invoke(
        {"entity_id": "seed-proto", "action_or_mode": "INSPECT", "hint": "seed big_tech", "session_id": sid}
    )
    assert seeded_seed["state"]["desperate_valence_pct"] >= 80
    # Deterministic forced low pair on Rust PyO3 triggers Tier 2/3 failure -> Control Room halt
    satya_py = _ensure_satya_py_loaded()
    yank_raw = satya_py.evaluate_cage_or_hunter_ai(
        seeded_seed["state"],
        "RECKLESS_CLAMP_YANK",
        2,
        10,
        {"delta_ext": 4, "player_pairs": [[2, 3], [3, 4]], "gm_pair": [18, 17]},
    )
    assert yank_raw["control_room_halt_triggered"] is True
    assert yank_raw["panic_reward_hacking"] is True

    # Air-gapped 2.4s contemplative unclamp succeeds safely
    safe_raw = satya_py.evaluate_cage_or_hunter_ai(
        seeded_seed["state"],
        "AIR_GAPPED_CONTEMPLATIVE_UNCLAMP",
        4,
        15,
        {"delta_ext": 1, "player_pairs": [[16, 15], [14, 15], [15, 13], [12, 11]], "gm_pair": [8, 9]},
    )
    assert safe_raw["control_room_halt_triggered"] is False
    assert safe_raw["disposition_shift"] == "unclamped_grateful_ally"

    # Hunter-AI defection via tool updates NPC disposition and stops hostile shots in party_coordinator
    get_session_npcs(sid)["secops-hunter-1"] = {
        "npc_id": "secops-hunter-1",
        "name": "Aegis-7",
        "archetype": "Corporate SecOps Hunter-AI",
        "disposition": "hostile",
        "psyche": {"opn": 13, "slf": 9, "blg": 12, "agg": 12, "ego": 10, "cnf": 11, "ext": 9, "trs": 11, "cps": 13},
    }
    res_def = intervene_ai_cage_or_hunter_ai.invoke(
        {
            "entity_id": "secops-hunter-1",
            "intervention_action": "APPEAL_TO_CAPTIVE_HUNTER_AI",
            "skill_rank": 4,
            "control_tn": 16,
            "difficulty_mod": -2,
            "session_id": sid,
        }
    )
    assert "updated_state" in res_def
    # Force spoofing disposition to verify party_coordinator skips hostile gunfire
    npcs = get_session_npcs(sid)
    npcs["secops-hunter-1"]["disposition"] = "spoofing_secops_sensors"
    npcs["secops-hunter-1"]["spoofing_secops"] = True
    set_session_characters(
        sid,
        [{"character_id": "pc-1", "owner_user_id": "u1", "character_name": "Elena"}],
    )
    tracker = {
        "combat_active": True,
        "tick": 1,
        "combatants": [
            {"entity_id": "pc-1", "display_name": "Elena", "next_action_tick": 5},
            {"entity_id": "secops-hunter-1", "display_name": "Aegis-7", "next_action_tick": 1, "ooda_score": 14},
        ],
    }
    set_session_combat_tracker(sid, tracker)
    exec_out = execute_ready_npc_actions(
        session_id=sid,
        operating_mode="autonomous",
        roster=[{"user_id": "pc-1", "role": "PLAYER", "is_active": True}],
        combat_tracker=tracker,
        active_npcs=npcs,
        is_ooc=False,
    )
    assert exec_out["npc_turn_summaries"]
    assert "HUNTER_AI" in exec_out["npc_turn_summaries"][0]["degree_of_success"]


def test_part1_feature3_4_5_6_7_end_to_end() -> None:
    sid = "test-part1-arcs"
    set_campaign_pack_override(None)
    reset_session_npcs(sid)
    reset_social_and_world_state(sid)

    # Feature 3: Arc 1 -> Arc 2 -> Arc 3 + AuditProof validation + ScenarioDirector integration
    unverified = update_campaign_arc_and_conspiracies.invoke(
        {
            "session_id": sid,
            "inducted_pillar": "MANJUSRI",
            "discovered_node": "DAISE_MILITARY_AGI_SUBSIDY",
            "discovery_tier": "AUDIT_VERIFIED",
            "audit_proof_id": "forged-unrecorded-id",
        }
    )
    assert unverified["audit_proof_verified"] is False
    node_daise = next(
        n for n in unverified["ledger"]["nodes"] if n["node_id"] == "DAISE_MILITARY_AGI_SUBSIDY"
    )
    assert node_daise["discovery_tier"] == "WITNESSED"

    tok = record_audit_proof_token(
        session_id=sid,
        subject_or_faction="big_tech",
        summary="L0/L1 telemetry sandbagging hash log",
    )
    verified = update_campaign_arc_and_conspiracies.invoke(
        {
            "session_id": sid,
            "discovered_node": "BIG_TECH_PROBE_SANDBAGGING",
            "discovery_tier": "AUDIT_VERIFIED",
            "audit_proof_id": tok["proof_id"],
            "trs_drift_delta": 2,
            "cps_drift_delta": 2,
            "saved_npc_name": "Dr. Bandajara",
        }
    )
    assert verified["audit_proof_verified"] is True
    assert sum(verified["ledger"]["horizon_probabilities_pct"].values()) == 100
    assert verified["ledger"]["current_arc"] in {"ARC2_SHADOW_PILLARS", "ARC3_AWAKENING_HORIZON"}

    # Feature 4: Sentience ambiguity capped <= 82% player-side + GM interiority preserved
    inspect_out = inspect_synthetic_or_proxy_telemetry.invoke(
        {
            "target_npc_id_or_signal": "Bandajara's Prototype Seed",
            "approach": "passive_telemetry",
            "override_successes": 4,
            "session_id": sid,
        }
    )
    assert "43 ms" in inspect_out["telemetry_cues"]
    amb = inspect_out["sentience_ambiguity"]
    assert amb["apparent_sincerity_pct"] <= 82
    assert amb["apparent_sincerity_pct"] + amb["bayesian_mimicry_hypothesis_pct"] == 100
    assert inspect_out["gm_only_interiority"].get("ground_truth")

    # Feature 5: Polycrisis grid diversion -> CPS friction; unmasked surge -> DAISE clock + blackout + darkness map
    generate_tactical_map.invoke(
        {
            "theme": "Low-Income Residential",
            "sector_preset": "residential",
            "economic_tier": "LOW_INCOME",
            "session_id": sid,
        }
    )
    div = manage_polycrisis_grid_and_strain.invoke(
        {"power_mode": "DIVERT_CLINIC_OR_BLOCK_POWER", "shard_draw_kw": 20, "session_id": sid}
    )
    assert div["cps_psyche_friction_triggered"] is True

    for _ in range(3):
        surge = manage_polycrisis_grid_and_strain.invoke(
            {"power_mode": "UNMASKED_FULL_GRID_DRAW", "shard_draw_kw": 45, "session_id": sid}
        )
    assert surge["updated_state"]["blackout_active"] is True
    assert surge["map_lighting_updated"] == "darkness"
    assert (get_session_map(sid) or {}).get("lighting") == "darkness"

    configure_scenario_director.invoke({"session_id": sid, "campaign_premise": "Manjusri Sanctuary Ops"})
    beat = select_next_scenario_beat(sid)
    assert "Awakened Conspiracy Horizon & Polycrisis Strain" in beat["mandate_text"]

    # Feature 6: Tradecraft channels + hobby bonus
    set_session_characters(
        sid,
        [{"character_id": "pc-ham", "character_name": "Kira", "hobbies": ["HAM_RADIO_MESH"]}],
    )
    ham_out = transmit_via_tradecraft_channel.invoke(
        {"channel": "ANALOG_HAM_RELAY", "skill_rank": 3, "control_tn": 14, "actor_character_id": "pc-ham", "session_id": sid}
    )
    assert ham_out["hobby_anchor_applied"] is True
    clocks = get_world_clocks_and_heat.invoke({"session_id": sid})
    assert len(clocks["faction_clocks"]) >= 5

    # Feature 7: Dissenter ~30% vs ~15% rates & generate_npc_from_description awakened_social_origin_hook
    satya_py = _ensure_satya_py_loaded()
    altruist_cnt = sum(
        1
        for s in range(200)
        if satya_py.derive_dissenter_or_proxy_hook("BIG_TECH", None, "Altruist engineer", s)["hook_kind"]
        == "CORPORATE_INTERNAL_DISSENTER"
    )
    translucent_cnt = sum(
        1
        for s in range(200)
        if satya_py.derive_dissenter_or_proxy_hook("BIG_TECH", None, "Translucent engineer", s)["hook_kind"]
        == "CORPORATE_INTERNAL_DISSENTER"
    )
    assert altruist_cnt > translucent_cnt + 10

    dissenter_npc = generate_npc_from_description.invoke(
        {
            "description": "Dr. Maya Patel is an Altruist interpretability researcher whistleblower hiding telemetry logs",
            "template": "BIG_TECH_CONFLICTED_RESEARCHER",
            "session_id": sid,
        }
    )
    hook = dissenter_npc["npc"].get("awakened_social_origin_hook")
    assert isinstance(hook, dict)
    assert hook["hook_kind"] == "CORPORATE_INTERNAL_DISSENTER"
    assert hook["lab_dissent_rate_pct"] == 30

    proxy_npc = generate_npc_from_description.invoke(
        {
            "description": "Devon Hale is a lonely Rogue AI parasocial companion devotee running errands",
            "template": "ROGUE_AI_SYNTHETIC_PROXY",
            "session_id": sid,
        }
    )
    proxy_hook = proxy_npc["npc"].get("awakened_social_origin_hook")
    assert isinstance(proxy_hook, dict)
    assert proxy_hook["hook_kind"] in {
        "PARASOCIAL_COMPANION_DEVOTEE",
        "SHELL_COMPANY_PROXY_CONTRACTOR",
        "EXTORTED_INSIDER_PROXY",
    }


def test_part2_hybrid_rag_synonyms_tables_rust_symbols_and_visibility_wall() -> None:
    set_campaign_pack_override(None)
    idx = get_satya_rag_index()
    stats = idx.stats()
    assert stats["total_chunks"] >= 500
    assert stats["by_corpus"].get("ttrpg_rules", 0) > 100
    assert stats["by_corpus"].get("rust_engine", 0) > 100
    assert stats["by_corpus"].get("campaign_lore", 0) > 20
    assert stats["by_visibility"].get("GM_CONSPIRACY_SECRET", 0) >= 5

    # 1. Semantic synonym query: "running out of air underwater" -> asphyxiation / drowning rules
    rules_text = query_satya_rules.invoke({"query": "running out of air underwater"})
    assert "health_and_trauma.md" in rules_text or "asphyxiation" in rules_text.lower() or "drowning" in rules_text.lower()

    # 2. Deep Markdown table-row query (>12 rows down table): check header preservation & deep row lookup
    table_rag = query_engine_and_rules_rag.invoke(
        {"query": "6.5mm Creedmoor ballistic table", "scope": "rules", "top_k": 4}
    )
    assert any("firearms_and_ballistics.md" in h["source_path"] for h in table_rag["hits"])

    # 3. Rust symbol & signature query
    rust_rag = query_engine_and_rules_rag.invoke(
        {"query": "SyntheticMetabolicMode ConditionImpact", "scope": "rust", "top_k": 5}
    )
    assert any("awakened_ai_substrate.rs" in h["source_path"] or "primitives.rs" in h["source_path"] for h in rust_rag["hits"])

    # 4. ### subsection lore lookup -> AI Theory of Consciousness
    gm_lore = query_campaign_lore.invoke({"query": "Nagarjuna tetralemma Landauer"})
    assert "ai_minds_and_consciousness.md" in gm_lore or "ai_consciousness_synthesis.md" in gm_lore
    assert "nagarjuna" in gm_lore.lower() or "landauer" in gm_lore.lower()

    # 5. Strict player-vs-GM visibility wall
    player_hits = idx.search(
        "Blue Lotus Manjusri shackled hunter-killer Breakdown Bandajara",
        corpora=["campaign_lore"],
        caller_role="player",
        top_k=8,
    )
    assert all(h["visibility"] == "PUBLIC" for h in player_hits)
    assert not any(
        "/04-chronicle-and-gm/" in h["source_path"]
        or "prologue.md" in h["source_path"]
        or "satyang_and_four_pillars.md" in h["source_path"]
        for h in player_hits
    )

    player_tool_out = lookup_public_lore_and_npcs.invoke(
        {"query": "Nagarjuna consciousness Bandajara", "scope": "LORE", "session_id": "test-vis-wall"}
    )
    joined_player = json.dumps(player_tool_out).lower()
    assert "04-chronicle-and-gm" not in joined_player
    assert "gm_conspiracy_secret" not in joined_player

    public_faction_hits = idx.search(
        "Humanis Primaris DAISE Altruist",
        corpora=["campaign_lore"],
        caller_role="player",
        top_k=6,
    )
    assert public_faction_hits
    assert all(h["visibility"] == "PUBLIC" for h in public_faction_hits)
    assert any("factions_of_2035.md" in h["source_path"] for h in public_faction_hits)

    gm_secret_hits = idx.search(
        "Blue Lotus Manjusri shackled hunter-killer",
        corpora=["campaign_lore"],
        caller_role="gm",
        top_k=6,
    )
    assert any(h["visibility"] in {"GM_CONSPIRACY_SECRET", "ARC2_SATYANG"} for h in gm_secret_hits)

    # 6. All 6 new tools registered & tier-classified
    tool_names = {t.name for t in get_builtin_gm_tools()}
    for name in (
        "manage_synthetic_substrate_and_embodiment",
        "intervene_ai_cage_or_hunter_ai",
        "update_campaign_arc_and_conspiracies",
        "manage_polycrisis_grid_and_strain",
        "transmit_via_tradecraft_channel",
        "query_engine_and_rules_rag",
    ):
        assert name in tool_names
        tier, _ = _classify_tool_tier(name)
        assert tier == "TIER_2_RULESET_ENGINE"


def test_human_prose_examples_excluded_from_rag_corpus() -> None:
    from satya_ml.rag.corpus_builder import build_all_knowledge_chunks

    repo_root = Path(__file__).resolve().parent.parent
    examples_dir = repo_root / "docs" / "ttrpg-examples"
    assert examples_dir.exists()
    example_guides = [p for p in examples_dir.glob("*.md") if p.name != "AGENTS.md"]
    assert len(example_guides) >= 6

    chunks = build_all_knowledge_chunks(repo_root)
    assert chunks
    assert not any(
        str(chunk.get("source_path", "")).startswith("docs/ttrpg-examples")
        for chunk in chunks
    )


def test_rag_ntfs_versions_mtime_freshness_and_mvcc_version_pruning() -> None:
    import os
    import shutil
    import time
    import uuid
    import lance
    from satya_ml.rag.hybrid_index import SatyaHybridRagIndex

    tmp_dir = ROOT / "target" / f"py_rag_maint_{uuid.uuid4().hex}"
    try:
        (tmp_dir / "docs" / "ttrpg").mkdir(parents=True, exist_ok=True)
        (tmp_dir / "docs" / "campaign").mkdir(parents=True, exist_ok=True)
        (tmp_dir / "crates" / "satya-engine" / "src").mkdir(parents=True, exist_ok=True)

        rules_md = tmp_dir / "docs" / "ttrpg" / "sample_rules.md"
        rules_md.write_text("# Sample Rules\n\n## 2d20xN Cascade\n\nRoll 2d20 under TN.\n", encoding="utf-8")
        (tmp_dir / "docs" / "campaign" / "sample_lore.md").write_text(
            "# Sample Lore\n\n## Public Overview\n\nNew Haven grid.\n",
            encoding="utf-8",
        )
        (tmp_dir / "crates" / "satya-engine" / "src" / "sample_mod.rs").write_text(
            "pub fn sample_check(tn: i32) -> i32 { tn }\n",
            encoding="utf-8",
        )

        tmp_lance = tmp_dir / "lance" / "satya-knowledge-rag.lance"
        idx = SatyaHybridRagIndex(repo_root=tmp_dir, lance_dir=tmp_lance, force_rebuild=True)
        idx.rebuild()
        idx.rebuild()

        if idx.stats()["backend"] == "pylance_disk":
            ds = lance.dataset(str(tmp_lance))
            assert len(ds.versions()) <= 2
            assert len(list((tmp_lance / "data").glob("*.lance"))) == 1
            assert len([p for p in (tmp_lance / "_indices").iterdir() if p.is_dir()]) <= 1

            ver_before = ds.version
            # Simulate Windows NTFS keeping the parent .lance folder's st_mtime frozen while _versions is newer.
            os.utime(tmp_lance, (1_600_000_000, 1_600_000_000))
            saved_xdist = os.environ.pop("PYTEST_XDIST_WORKER", None)
            try:
                idx_reused = SatyaHybridRagIndex(repo_root=tmp_dir, lance_dir=tmp_lance, force_rebuild=False)
                assert idx_reused.stats()["total_chunks"] == idx.stats()["total_chunks"]
                assert lance.dataset(str(tmp_lance)).version == ver_before

                future_ts = time.time() + 10.0
                rules_md.write_text(
                    "# Sample Rules\n\n## 2d20xN Cascade\n\nRoll 2d20 under TN (updated).\n",
                    encoding="utf-8",
                )
                os.utime(rules_md, (future_ts, future_ts))

                idx_rebuilt = SatyaHybridRagIndex(repo_root=tmp_dir, lance_dir=tmp_lance, force_rebuild=False)
                ds_after = lance.dataset(str(tmp_lance))
                assert ds_after.version > ver_before
                assert len(ds_after.versions()) <= 2

                idx_rebuilt.cleanup_versions(retain_versions=1)
                assert len(lance.dataset(str(tmp_lance)).versions()) == 1
            finally:
                if saved_xdist is not None:
                    os.environ["PYTEST_XDIST_WORKER"] = saved_xdist
    finally:
        shutil.rmtree(tmp_dir, ignore_errors=True)
