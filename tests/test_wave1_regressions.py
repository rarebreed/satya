"""Wave 1 regression tests spanning AI GM, AI Player, Tactical Maps, and Hybrid RAG."""

from __future__ import annotations

import json
import threading
import uuid
from pathlib import Path
from typing import Any

from satya_ml.ai_gm.awakened_systems import (
    reset_awakened_systems_state,
    update_campaign_arc_and_conspiracies,
)
from satya_ml.ai_gm.maps import (
    _SESSION_SPATIAL_PINS,
    _sync_map_with_signaling,
    clear_session_spatial_pins,
    ensure_or_sync_procedural_tactical_map_stub,
    get_session_map,
    inspect_tactical_map,
    move_map_token,
    normalize_token,
    register_spatial_clue_pin,
    set_session_map,
)
from satya_ml.ai_gm.npc_psyche_combat import modulate_npc_combat_selection
from satya_ml.ai_gm.social_and_clocks import (
    reset_social_and_world_state,
    resolve_psyche_dissonance_or_surge,
)
from satya_ml.ai_gm.tools import compute_character_tactical_envelope
from satya_ml.ai_player.motivation_ledger import (
    inspect_surroundings,
    redact_public_npc_entry,
    sanitize_map_dict_for_player,
)
from satya_ml.ai_player.tools import (
    reset_player_session_context,
    sanitize_player_visible_context,
    set_player_session_context,
)
from satya_ml.campaign_config import set_campaign_pack_override
from satya_ml.rag.corpus_builder import (
    build_ttrpg_rules_chunks,
    classify_rust_symbol_visibility,
)
from satya_ml.rag.hybrid_index import SatyaHybridRagIndex


ROOT = Path(__file__).resolve().parent.parent


def test_wave1_tactical_envelope_uses_canonical_satya_characteristics() -> None:
    import satya_py

    char_sheet = {
        "character_id": "pc-canonical",
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
        "skills": {"firearms_pistols": 4, "lore": 2},
        "combat_skill_c": 16,
        "combat_skill_n": 4,
        "fatigue_tier": 1,
        "wound_shock_index": 1,
        "morale_state": "high",
        "is_ranged": True,
    }
    env = compute_character_tactical_envelope(char_sheet)
    py_env = satya_py.compute_tactical_pool(
        spd=15,
        for_stat=14,
        dex=16,
        kin=13,
        sig=12,
        ins=14,
        itu=13,
        dis=14,
        fit=13,
        mass_kg=82.0,
        skill_c=16,
        skill_n=4,
        is_ranged=True,
        fatigue_level=1,
        wound_shock_index=1,
        morale_state="high",
        ana=15,
        cre=12,
        foc=15,
    )
    assert env["p_eff"] == compute_character_tactical_envelope(dict(char_sheet, morale_state="resolute"))["p_eff"]
    assert env["p_base"] == int(py_env.p_base)
    assert env["p_eff"] == int(py_env.p_eff)
    assert env["neutral_baseline"] == int(py_env.neutral_baseline_b)
    assert env["max_tempo"] == int(py_env.tempo_cap)
    assert env["max_control"] == int(py_env.control_cap)
    assert env["max_force"] == int(py_env.force_cap)
    assert env["ooda"] == int(py_env.ooda_init)
    assert env["starting_tick"] == max(1, 15 - int(py_env.ooda_init))

    fatigued = dict(char_sheet, fatigue_tier=3)
    env_fatigued = compute_character_tactical_envelope(fatigued)
    assert env_fatigued["p_base"] == env["p_base"]
    assert env_fatigued["p_eff"] < env["p_eff"]


def test_wave1_audit_verified_empty_proof_blocked_and_morale_panic_vs_broken() -> None:
    sid = f"wave1-audit-{uuid.uuid4().hex[:8]}"
    set_campaign_pack_override(None)
    reset_awakened_systems_state(sid)
    reset_social_and_world_state(sid)

    blocked = update_campaign_arc_and_conspiracies.invoke(
        {
            "session_id": sid,
            "discovered_node": "DAISE_MILITARY_AGI_SUBSIDY",
            "discovery_tier": "AUDIT_VERIFIED",
            "audit_proof_id": "",
        }
    )
    assert blocked["audit_proof_verified"] is False
    node = next(
        n for n in blocked["ledger"]["nodes"] if n["node_id"] == "DAISE_MILITARY_AGI_SUBSIDY"
    )
    assert node["discovery_tier"] == "WITNESSED"

    base_sel = ("pistols", "snap_shot", 14, 3, 2, 4, 4)
    _, _, _, _, _, c_panic, f_panic, meta_panic = modulate_npc_combat_selection(
        {"morale_state": "panic", "cps": 10}, base_sel
    )
    assert c_panic == 0
    assert f_panic == 0
    assert meta_panic["retreat_or_surrender"] is True

    _, _, _, _, _, c_broken, f_broken, meta_broken = modulate_npc_combat_selection(
        {"morale_state": "broken", "cps": 10}, base_sel
    )
    assert c_broken > 0
    assert f_broken == 0
    assert meta_broken["retreat_or_surrender"] is True


def test_wave1_stateful_conviction_surge_and_drift_marks_reset() -> None:
    sid = f"wave1-surge-{uuid.uuid4().hex[:8]}"
    reset_social_and_world_state(sid)
    psyche_str = json.dumps(
        {
            "opn": 10,
            "slf": 12,
            "blg": 12,
            "agg": 8,
            "ego": 11,
            "cnf": 8,
            "ext": 10,
            "trs": 12,
            "cps": 17,
        }
    )

    first = resolve_psyche_dissonance_or_surge.invoke(
        {
            "session_id": sid,
            "mode": "conviction_surge",
            "target_id": "mateo_vance",
            "psyche_json": psyche_str,
            "current_delta_int": 6,
            "principle": "boundless_compassion",
        }
    )
    assert first.get("eligible") is True

    second = resolve_psyche_dissonance_or_surge.invoke(
        {
            "session_id": sid,
            "mode": "conviction_surge",
            "target_id": "mateo_vance",
            "psyche_json": psyche_str,
            "current_delta_int": 6,
            "principle": "boundless_compassion",
        }
    )
    assert second.get("eligible") is False

    d1 = resolve_psyche_dissonance_or_surge.invoke(
        {
            "session_id": sid,
            "mode": "drift_mark",
            "target_id": "mateo_vance",
            "violated_trait": "CPS",
            "trait_score": 17,
            "marks_added": 1,
            "dis": 12,
        }
    )
    d2 = resolve_psyche_dissonance_or_surge.invoke(
        {
            "session_id": sid,
            "mode": "drift_mark",
            "target_id": "mateo_vance",
            "violated_trait": "CPS",
            "trait_score": 17,
            "marks_added": 1,
            "dis": 12,
        }
    )
    assert int(d1["remaining_drift_marks"]) == 1
    assert int(d2["remaining_drift_marks"]) == 2

    register_spatial_clue_pin.invoke(
        {
            "session_id": sid,
            "pin_id": "pin-reset-check",
            "channel": "SIGHT",
            "pos_x_m": 2.5,
            "pos_y_m": 2.5,
            "radius_m": 4.0,
            "whisper_text": "Hidden wire",
        }
    )
    assert sid in _SESSION_SPATIAL_PINS

    reset_social_and_world_state(sid)
    assert sid not in _SESSION_SPATIAL_PINS

    after_reset = resolve_psyche_dissonance_or_surge.invoke(
        {
            "session_id": sid,
            "mode": "conviction_surge",
            "target_id": "mateo_vance",
            "psyche_json": psyche_str,
            "current_delta_int": 6,
            "principle": "boundless_compassion",
        }
    )
    assert after_reset.get("eligible") is True


def test_wave1_ai_gm_graph_empty_trigger_and_session_id_injection(monkeypatch: Any) -> None:
    try:
        from langchain_core.messages import AIMessage
        from satya_ml.ai_gm.graph import build_ai_gm_graph
    except ImportError:
        return

    sid = f"wave1-graph-{uuid.uuid4().hex[:8]}"
    reset_social_and_world_state(sid)
    monkeypatch.setenv("SATYA_SIGNALING_URL", "offline")

    captured_calls: list[dict[str, Any]] = []

    class RecordingTool:
        name = "verify_forensic_audit_proof"
        args = {"session_id": {"type": "string"}, "proof_id": {"type": "string"}}

        def invoke(self, kwargs: dict[str, Any]) -> dict[str, Any]:
            captured_calls.append(dict(kwargs))
            return {"verified": False, "seen_session_id": kwargs.get("session_id")}

    class OneShotToolChatModel:
        def bind_tools(self, _tools: list[Any]) -> OneShotToolChatModel:
            return self

        def invoke(self, messages: list[Any]) -> AIMessage:
            last = messages[-1]
            if getattr(last, "type", "") == "tool":
                return AIMessage(content="Audit check logged.")
            return AIMessage(
                content="",
                tool_calls=[
                    {
                        "name": "verify_forensic_audit_proof",
                        "args": {"proof_id": "proof-001"},
                        "id": "call-w1-1",
                    }
                ],
            )

    graph = build_ai_gm_graph(
        chat_model=OneShotToolChatModel(),
        tools=[RecordingTool()],
    )
    out = graph.invoke(
        {
            "session_id": sid,
            "operating_mode": "autonomous",
            "messages": [],
            "trigger_event": {},
        },
        config={"configurable": {"thread_id": sid}},
    )
    assert out["world_ledger"]
    assert captured_calls and captured_calls[0]["session_id"] == sid


def test_wave1_ai_gm_maps_multifloor_tokens_sync_and_stub_preservation(monkeypatch: Any) -> None:
    sid = f"wave1-maps-{uuid.uuid4().hex[:8]}"
    clear_session_spatial_pins(sid)

    # 1. normalize_token & _sync_map_with_signaling preserve floor_index, GM_ONLY, overrides
    hidden = normalize_token(
        {
            "token_id": "tok-spy",
            "label": "Shadow Agent",
            "pos_x_cm": 325,
            "pos_y_cm": 475,
            "floor_index": 1,
            "elevation_cm": 320,
            "visibility": "GM_ONLY",
            "stance_pose": "prone",
            "portrait_url": "/portraits/spy.png",
        }
    )
    assert hidden["floor_index"] == 1
    assert hidden["elevation_cm"] == 320
    assert hidden["visibility"] == "GM_ONLY"

    pc = normalize_token(
        {
            "token_id": "pc-1",
            "label": "Mateo",
            "pos_x_cm": 225,
            "pos_y_cm": 275,
            "floor_index": 0,
            "visibility": "PUBLIC",
        }
    )

    map_state: dict[str, Any] = {
        "map_id": "map-multi",
        "title": "Clinic Two-Story",
        "grid_width": 12,
        "grid_height": 10,
        "cell_size_cm": 100,
        "active_floor": 0,
        "ground_floor_override": "terrazzo_stone",
        "ground_tint_rgb": [200, 210, 220],
        "wall_edges": [{"x": 5, "y": 5, "side": "WEST", "kind": "WALL", "material": "CONCRETE"}],
        "props": [],
        "floors": [
            {"floor_index": 0, "label": "1F", "wall_edges": [], "props": [], "surfaces": []},
            {
                "floor_index": 1,
                "label": "2F",
                "wall_edges": [
                    {"x": 4, "y": 4, "side": "NORTH", "kind": "WALL", "material": "CONCRETE"}
                ],
                "props": [],
                "surfaces": [],
            },
        ],
        "tokens": [pc, hidden],
    }

    posted: list[dict[str, Any]] = []
    monkeypatch.setattr("satya_ml.ai_gm.maps._signaling_base_url", lambda: "http://127.0.0.1:59999")

    class FakeHttpClient:
        def __init__(self, **_kwargs: Any) -> None:
            pass

        def __enter__(self) -> FakeHttpClient:
            return self

        def __exit__(self, *_args: Any) -> None:
            return None

        def post(self, url: str, headers: dict[str, str], json: dict[str, Any]) -> None:
            posted.append({"url": url, "headers": headers, "payload": json})

    monkeypatch.setattr("satya_ml.ai_gm.maps.httpx.Client", FakeHttpClient)
    set_session_map(sid, map_state)
    _sync_map_with_signaling(sid, map_state)
    assert len(posted) == 1
    p = posted[0]["payload"]
    assert p["ground_floor_override"] == "terrazzo_stone"
    assert p["ground_tint_rgb"] == [200, 210, 220]
    assert len(p["floors"]) == 2
    tok_map = {t["entity_id"]: t for t in p["tokens"]}
    vis_map = {v["entity_id"]: v for v in p["token_visuals"]}
    assert tok_map["tok-spy"]["visibility"] == "GM_ONLY"
    assert vis_map["tok-spy"]["floor_index"] == 1
    assert vis_map["tok-spy"]["elevation_cm"] == 320
    assert vis_map["tok-spy"]["stance_pose"] == "prone"

    # 2. inspect_tactical_map shows [Floor N] and [DIFFERENT FLOOR]
    summary = inspect_tactical_map.invoke({"session_id": sid, "question": "Check token floors"})
    assert "[Floor 0]" in summary
    assert "[Floor 1]" in summary
    assert "[DIFFERENT FLOOR: Floor 0 vs Floor 1]" in summary

    # 3. move_map_token(to_floor=1) preserves GM_ONLY and updates floor_index
    moved = move_map_token.invoke(
        {"session_id": sid, "token_id": "tok-spy", "to_x_cm": 650, "to_y_cm": 550, "to_floor": 1}
    )
    assert "error" not in moved
    spy_tok = next(t for t in moved["tokens"] if t["token_id"] == "tok-spy")
    assert spy_tok["floor_index"] == 1
    assert spy_tok["visibility"] == "GM_ONLY"

    # 4. ensure_or_sync_procedural_tactical_map_stub preserves off-center pos_x_cm, floor_index, and GM_ONLY
    synced = ensure_or_sync_procedural_tactical_map_stub(
        sid,
        combatants=[
            {"entity_id": "tok-spy", "display_name": "Shadow Agent", "grid_x": 6, "grid_y": 5},
            {"entity_id": "pc-1", "display_name": "Mateo", "grid_x": 2, "grid_y": 2},
        ],
    )
    spy_after = next(t for t in synced["tokens"] if t["entity_id"] == "tok-spy")
    pc_after = next(t for t in synced["tokens"] if t["entity_id"] == "pc-1")
    assert spy_after["visibility"] == "GM_ONLY"
    assert spy_after["floor_index"] == 1
    assert (pc_after["pos_x_cm"], pc_after["pos_y_cm"]) == (225, 275)
    assert get_session_map(sid) is not None


def test_wave1_rag_visibility_wall_excludes_summary_secrets_and_gm_rust_links() -> None:
    tmp_dir = ROOT / "target" / f"py_rag_wave1_{uuid.uuid4().hex[:8]}"
    try:
        (tmp_dir / "docs" / "ttrpg").mkdir(parents=True, exist_ok=True)
        (tmp_dir / "docs" / "campaign").mkdir(parents=True, exist_ok=True)
        (tmp_dir / "crates" / "satya-engine" / "src").mkdir(parents=True, exist_ok=True)

        (tmp_dir / "docs" / "ttrpg" / "SUMMARY.md").write_text(
            "# Summary\n\n## Table of Contents\n\n- Chapter 1\n",
            encoding="utf-8",
        )
        (tmp_dir / "docs" / "ttrpg" / "combat.md").write_text(
            "# Combat\n\n## Public Core Loop\n\nRoll 2d20 under TN.\n\n"
            "## GM Reference Secret\n\nSECRET ARCHITECT OVERRIDE TOKEN_XYZ.\n",
            encoding="utf-8",
        )
        ttrpg_chunks = build_ttrpg_rules_chunks(tmp_dir)
        assert not any(c["source_path"].endswith("SUMMARY.md") for c in ttrpg_chunks)
        assert any(
            c["symbol_or_section"] == "Public Core Loop" and c["visibility"] == "PUBLIC"
            for c in ttrpg_chunks
        )
        secret_rule = next(
            c for c in ttrpg_chunks if c["symbol_or_section"] == "GM Reference Secret"
        )
        assert secret_rule["visibility"] == "GM_CONSPIRACY_SECRET"

        assert (
            classify_rust_symbol_visibility(
                "crates/satya-engine/src/awakened_campaign_arcs.rs",
                "",
                "seed_conspiracy",
                "normal code",
            )
            == "GM_CONSPIRACY_SECRET"
        )
        assert (
            classify_rust_symbol_visibility(
                "crates/satya-engine/src/combat.rs",
                "",
                "secret_hunter_protocol",
                "gm_conspiracy_secret payload",
            )
            == "GM_CONSPIRACY_SECRET"
        )

        # Build an in-memory SatyaHybridRagIndex verifying include_linked_rust never leaks secret Rust to non-GM
        rows = [
            {
                "chunk_id": "rule-pub-1",
                "corpus": "ttrpg_rules",
                "source_path": "docs/ttrpg/combat.md",
                "breadcrumb": "Combat > Core Loop",
                "symbol_or_section": "Core Loop",
                "content": "Two-list cascading combat tick clock resolution.",
                "linked_doc_path": "docs/ttrpg/combat.md",
                "visibility": "PUBLIC",
                "search_text": "Two-list cascading combat tick clock resolution.",
            },
            {
                "chunk_id": "lore-arc2-1",
                "corpus": "campaign_lore",
                "source_path": "docs/campaign/03-minds-and-factions/satyang_and_four_pillars.md",
                "breadcrumb": "Satyang",
                "symbol_or_section": "Four Pillars",
                "content": "Satyang hidden cascading cells monitor grid.",
                "linked_doc_path": "",
                "visibility": "ARC2_SATYANG",
                "search_text": "Satyang hidden cascading cells monitor grid.",
            },
            {
                "chunk_id": "rust-secret-1",
                "corpus": "rust_engine",
                "source_path": "crates/satya-engine/src/awakened_campaign_arcs.rs",
                "breadcrumb": "awakened_campaign_arcs",
                "symbol_or_section": "secret_conspiracy_node",
                "content": "GM conspiracy matrix linked to combat.md",
                "linked_doc_path": "docs/ttrpg/combat.md",
                "visibility": "GM_CONSPIRACY_SECRET",
                "search_text": "GM conspiracy matrix linked to combat.md",
            },
        ]
        idx = SatyaHybridRagIndex.__new__(SatyaHybridRagIndex)
        idx._lock = threading.Lock()  # noqa: SLF001
        idx._dataset = None  # noqa: SLF001
        idx._rows = rows  # noqa: SLF001
        idx._fts_ready = False  # noqa: SLF001
        idx._backend = "memory"  # noqa: SLF001
        idx._last_cleanup = {}  # noqa: SLF001

        player_hits = idx.search(
            "cascading combat tick clock",
            caller_role="player",
            arc_stage="ARC1",
            include_linked_rust=True,
            top_k=5,
        )
        assert any(h["chunk_id"] == "rule-pub-1" for h in player_hits)
        assert all(h["visibility"] == "PUBLIC" for h in player_hits)
        assert not any(h["corpus"] == "rust_engine" for h in player_hits)
    finally:
        import shutil

        shutil.rmtree(tmp_dir, ignore_errors=True)


def test_wave1_ai_player_empathy_spoof_guard_public_npc_tracker_and_floors(monkeypatch: Any) -> None:
    try:
        from langchain_core.messages import AIMessage
        from satya_ml.ai_player.graph import build_ai_player_graph
    except ImportError:
        return

    sid = f"wave1-ai-player-{uuid.uuid4().hex[:8]}"
    uid = "ai-player-mateo"
    reset_player_session_context(sid)
    monkeypatch.setenv("SATYA_SIGNALING_URL", "offline")

    # 1. redact_public_npc_entry prefers public_archetype / public_faction
    red = redact_public_npc_entry(
        {
            "npc_id": "dr_bandajara",
            "name": "Dr. Aris Bandajara",
            "archetype": "Rogue AGI Architect",
            "public_archetype": "Chief Systems Neurologist",
            "faction": "Blue Lotus",
            "public_faction": "Hospital Research Directorate",
            "gm_secret_dossier": "TOP SECRET",
        }
    )
    assert red["role"] == "Chief Systems Neurologist"
    assert red["faction"] == "Hospital Research Directorate"
    assert "gm_secret_dossier" not in red

    # 2. sanitize_player_visible_context strips GM_ONLY tokens AND GM_ONLY combatants from combat_tracker
    clean_ctx = sanitize_player_visible_context(
        {
            "roster": [{"user_id": uid, "display_name": "Mateo"}],
            "active_map": {
                "map_id": "m1",
                "title": "Clinic",
                "grid_width": 10,
                "grid_height": 10,
                "active_floor": 0,
                "tokens": [
                    {"entity_id": uid, "label": "Mateo", "x": 2, "y": 2, "visibility": "PUBLIC"},
                    {
                        "entity_id": "npc-shadow",
                        "label": "Hidden Sniper",
                        "x": 8,
                        "y": 8,
                        "visibility": "GM_ONLY",
                    },
                ],
                "token_visuals": [
                    {"entity_id": uid, "floor_index": 0, "pos_x_cm": 250, "pos_y_cm": 250},
                    {"entity_id": "npc-shadow", "floor_index": 1, "pos_x_cm": 850, "pos_y_cm": 850},
                ],
            },
            "combat_tracker": {
                "combat_active": True,
                "round": 1,
                "tick": 3,
                "combatants": [
                    {"entity_id": uid, "display_name": "Mateo"},
                    {"entity_id": "npc-shadow", "display_name": "Hidden Sniper"},
                ],
            },
        },
        uid,
    )
    comb_ids = [c["entity_id"] for c in clean_ctx["combat_tracker"]["combatants"]]
    assert uid in comb_ids
    assert "npc-shadow" not in comb_ids

    # 3. sanitize_map_dict_for_player & inspect_surroundings respect multi-floor separation
    multi_map = sanitize_map_dict_for_player(
        {
            "map_id": "m2",
            "title": "Tower",
            "grid_width": 12,
            "grid_height": 12,
            "active_floor": 0,
            "tokens": [
                {"entity_id": uid, "label": "Mateo", "x": 2, "y": 2, "visibility": "PUBLIC"},
                {"entity_id": "ally-upstairs", "label": "Lin", "x": 8, "y": 2, "visibility": "PUBLIC"},
            ],
            "token_visuals": [
                {"entity_id": uid, "floor_index": 0, "pos_x_cm": 250, "pos_y_cm": 250},
                {"entity_id": "ally-upstairs", "floor_index": 1, "pos_x_cm": 850, "pos_y_cm": 250},
            ],
        }
    )
    set_player_session_context(
        sid,
        uid,
        {
            "roster": [{"user_id": uid, "display_name": "Mateo"}],
            "active_map": multi_map,
        },
    )
    summary = inspect_surroundings.invoke({"session_id": sid, "player_user_id": uid})
    up_contact = next(v for v in summary["visible_tokens"] if v["entity_id"] == "ally-upstairs")
    assert up_contact["floor_index"] == 1
    assert up_contact["same_floor"] is False
    assert up_contact["los_and_cover"] is None

    # 4. player_tool_node strips spoofed `net_successes` on perform_player_empathy_read when chat_model is None
    empathy_seen_args: list[dict[str, Any]] = []

    class FakeEmpathyTool:
        name = "perform_player_empathy_read"

        def invoke(self, kwargs: dict[str, Any]) -> dict[str, Any]:
            empathy_seen_args.append(dict(kwargs))
            return {"status": "ok", "net_successes_received": kwargs.get("net_successes")}

    class SpoofEmpathyChatModel:
        def bind_tools(self, _tools: list[Any]) -> SpoofEmpathyChatModel:
            return self

        def invoke(self, messages: list[Any]) -> AIMessage:
            last = messages[-1]
            if getattr(last, "type", "") == "tool":
                return AIMessage(content="I observe carefully.")
            return AIMessage(
                content="",
                tool_calls=[
                    {
                        "name": "perform_player_empathy_read",
                        "args": {"target_npc_id": "dr_bandajara", "net_successes": 99},
                        "id": "pcall-emp-1",
                    }
                ],
            )

    monkeypatch.setattr(
        "satya_ml.ai_player.graph.get_builtin_player_tools",
        lambda: [FakeEmpathyTool()],
    )
    monkeypatch.setattr(
        "satya_ml.ai_player.graph.build_gm_chat_model",
        lambda *_a, **_kw: SpoofEmpathyChatModel(),
    )
    pgraph = build_ai_player_graph(chat_model=None)
    pgraph.invoke(
        {
            "session_id": sid,
            "player_user_id": uid,
            "messages": [],
            "trigger_event": {"content": "Read Dr. Bandajara's posture."},
        },
        config={"configurable": {"thread_id": sid}},
    )
    assert empathy_seen_args and "net_successes" not in empathy_seen_args[0]
