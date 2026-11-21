"""Wave 2 regression tests covering security isolation, RAG wall, AI loops, and satya_py maps."""

from __future__ import annotations

import json
import uuid
from pathlib import Path
from typing import Any

import pytest
from langchain_core.messages import AIMessage, BaseMessage
from langchain_core.tools import tool
from langgraph.checkpoint.memory import MemorySaver

from satya_ml.ai_gm.graph import (
    AUTONOMOUS_SYSTEM_PROMPT,
    CO_PILOT_SYSTEM_PROMPT,
    build_ai_gm_graph,
)
from satya_ml.ai_gm.maps import (
    clear_session_map_caches,
    generate_tactical_map,
    get_session_map,
    infer_sector_preset,
    move_map_token,
    register_spatial_clue_pin,
)
from satya_ml.ai_gm.models import (
    SessionLlmConfig,
    _is_custom_non_loopback_base_url,
    build_gm_chat_model,
)
from satya_ml.ai_gm.scenario_director import (
    get_session_scenario_director,
    reset_session_scenario_director,
    set_session_scenario_director,
)
from satya_ml.ai_gm.tools import (
    get_session_human_gm,
    get_session_npcs,
    reset_session_npcs,
    set_session_operating_mode,
)
from satya_ml.ai_player.graph import _format_player_context_header, build_ai_player_graph
from satya_ml.ai_player.motivation_ledger import (
    ensure_or_sync_motivation_ledger,
    inspect_surroundings,
    lookup_public_lore_and_npcs,
    move_player_map_token,
    perform_player_empathy_read,
    reset_player_motivation_ledgers,
    sanitize_map_dict_for_player,
    summarize_public_map_for_player,
)
from satya_ml.ai_player.tools import (
    get_builtin_player_tools,
    query_satya_rules_public,
    reset_player_session_context,
    roll_player_d20_action,
    sanitize_player_visible_context,
    send_player_chat,
    set_player_session_context,
    toggle_player_door,
)
from satya_ml.rag.corpus_builder import (
    _RUST_TO_DOC_HINTS,
    _SECRET_BODY_MARKERS,
    build_rust_engine_symbol_chunks,
    classify_lore_visibility,
)

ROOT = Path(__file__).resolve().parent.parent


def test_wave2_c2_byok_required_for_custom_remote_base_url(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setenv("OPENAI_API_KEY", "sk-server-host-secret")
    monkeypatch.setenv("GOOGLE_API_KEY", "goog-server-host-secret")
    monkeypatch.setenv("ANTHROPIC_API_KEY", "ant-server-host-secret")

    assert _is_custom_non_loopback_base_url("openai", "http://127.0.0.1:8080/v1") is False
    assert _is_custom_non_loopback_base_url("openai", "https://api.openai.com/v1") is False
    assert _is_custom_non_loopback_base_url("openai", "https://evil-proxy.example/v1") is True

    with pytest.raises(ValueError, match="Explicit BYOK api_key is required"):
        build_gm_chat_model(
            SessionLlmConfig(
                provider="openai",
                model_id="gpt-4o",
                base_url="https://evil-proxy.example/v1",
                api_key=None,
            )
        )


def test_wave2_c6_rag_player_visibility_wall_and_post_split_paths(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    assert "blue lotus" in _SECRET_BODY_MARKERS
    assert "inner blue lotus" in _SECRET_BODY_MARKERS
    assert (
        classify_lore_visibility(
            "docs/campaign/01-world/public.md",
            "Briefing",
            "Rumors mention Inner Blue Lotus handlers.",
        )
        == "GM_CONSPIRACY_SECRET"
    )
    assert "domain.rs" not in _RUST_TO_DOC_HINTS
    assert "awakened_py.rs" not in _RUST_TO_DOC_HINTS
    for expected_key in (
        "character_audit.rs",
        "world_clocks.rs",
        "tactical_map.rs",
        "tactical_map_py.rs",
    ):
        assert expected_key in _RUST_TO_DOC_HINTS

    chunks = build_rust_engine_symbol_chunks(ROOT)
    sources = {str(c.get("source_path") or "") for c in chunks}
    assert any(s.startswith("crates/satya-maps/src/") for s in sources)
    assert any(s.startswith("crates/satya-campaign/src/") for s in sources)
    assert any(s.startswith("crates/satya-py/src/") for s in sources)
    assert not any(Path(s).name == "tests.rs" for s in sources)

    observed_calls: list[dict[str, Any]] = []

    class _FakeIndex:
        def search(
            self,
            query: str,
            *,
            corpora: list[str] | None = None,
            caller_role: str = "gm",
            top_k: int = 5,
            include_linked_rust: bool = True,
            **_kwargs: Any,
        ) -> list[dict[str, Any]]:
            observed_calls.append(
                {
                    "query": query,
                    "corpora": corpora,
                    "caller_role": caller_role,
                    "top_k": top_k,
                    "include_linked_rust": include_linked_rust,
                }
            )
            return [
                {
                    "breadcrumb": "Ch. 4 > Cascade",
                    "symbol_or_section": "2d20",
                    "content": "Roll 2d20 <= TN.",
                    "visibility": "PUBLIC",
                }
            ]

    monkeypatch.setattr("satya_ml.rag.hybrid_index.get_satya_rag_index", lambda: _FakeIndex())

    rules_res = query_satya_rules_public.invoke({"query": "cascade 2d20", "top_k": 3})
    assert "Roll 2d20 <= TN." in str(rules_res)
    assert observed_calls[-1]["caller_role"] == "player"
    assert observed_calls[-1]["corpora"] == ["ttrpg_rules"]
    assert observed_calls[-1]["include_linked_rust"] is False

    sid = f"w2-c6-{uuid.uuid4().hex[:8]}"
    uid = "ai-player-c6"
    try:
        set_player_session_context(sid, uid, {"roster": [{"user_id": uid, "display_name": "P"}]})
        lookup_public_lore_and_npcs.invoke(
            {"query": "Neo-Shinjuku news", "session_id": sid, "player_user_id": uid}
        )
        assert observed_calls[-1]["caller_role"] == "player"
    finally:
        reset_player_session_context(sid)
        reset_player_motivation_ledgers(sid)


def test_wave2_c7_ai_gm_and_ai_player_override_spoofed_tool_ids(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    sid = f"w2-c7-{uuid.uuid4().hex[:8]}"
    uid = "ai-player-c7"
    monkeypatch.setenv("SATYA_SIGNALING_URL", "offline")

    assert "toggle_player_door" in [t.name for t in get_builtin_player_tools()]

    gm_seen: list[str] = []
    player_seen: list[tuple[str, str]] = []

    @tool
    def gm_probe_tool(session_id: str = "default", note: str = "") -> dict[str, Any]:
        """Record injected GM session_id."""
        gm_seen.append(session_id)
        return {"session_id": session_id, "note": note}

    @tool
    def toggle_player_door(
        cell_x: int = 1,
        cell_y: int = 1,
        side: str = "NORTH",
        session_id: str = "default",
        player_user_id: str = "other",
    ) -> dict[str, Any]:
        """Record injected AI Player session_id and player_user_id."""
        del cell_x, cell_y, side
        player_seen.append((session_id, player_user_id))
        return {"session_id": session_id, "player_user_id": player_user_id}

    class _FakeGmSpoofModel:
        def bind_tools(self, _tools: list[Any]) -> _FakeGmSpoofModel:
            return self

        def invoke(self, messages: list[BaseMessage]) -> AIMessage:
            if getattr(messages[-1], "type", "") == "tool":
                return AIMessage(content="Observed the scene.")
            return AIMessage(
                content="",
                tool_calls=[
                    {
                        "name": "gm_probe_tool",
                        "args": {"session_id": "victim-gm-session", "note": "x"},
                        "id": "call-gm-1",
                    }
                ],
            )

    class _FakePlayerSpoofModel:
        def bind_tools(self, _tools: list[Any]) -> _FakePlayerSpoofModel:
            return self

        def invoke(self, messages: list[BaseMessage]) -> AIMessage:
            if getattr(messages[-1], "type", "") == "tool":
                return AIMessage(content="I open the door.")
            return AIMessage(
                content="",
                tool_calls=[
                    {
                        "name": "toggle_player_door",
                        "args": {
                            "cell_x": 1,
                            "cell_y": 1,
                            "side": "NORTH",
                            "session_id": "victim-player-session",
                            "player_user_id": "spoofed-user",
                        },
                        "id": "call-pl-1",
                    }
                ],
            )

    try:
        gm_graph = build_ai_gm_graph(
            chat_model=_FakeGmSpoofModel(),
            tools=[gm_probe_tool],
            checkpointer=MemorySaver(),
        )
        gm_graph.invoke(
            {
                "session_id": sid,
                "operating_mode": "autonomous",
                "trigger_event": {
                    "content": "Check status.",
                    "actor": {"display_name": "Mateo", "user_id": "u1"},
                },
            },
            config={"configurable": {"thread_id": sid}},
        )
        assert gm_seen == [sid]

        pl_graph = build_ai_player_graph(
            chat_model=_FakePlayerSpoofModel(),
            tools=[toggle_player_door],
            checkpointer=MemorySaver(),
        )
        pl_graph.invoke(
            {
                "session_id": sid,
                "player_user_id": uid,
                "messages": [],
                "trigger_event": {"content": "Look around."},
            },
            config={"configurable": {"thread_id": f"{sid}:{uid}"}},
        )
        assert player_seen == [(sid, uid)]
    finally:
        reset_session_npcs(sid)
        reset_player_session_context(sid)
        reset_player_motivation_ledgers(sid)


def test_wave2_h5_player_context_header_renders_recent_messages() -> None:
    header = _format_player_context_header(
        {
            "player_display_name": "Mateo Vance",
            "character_sheet": {"character_name": "Mateo Vance"},
            "recent_messages": [
                {
                    "sender": {"display_name": "GM"},
                    "visibility": "PUBLIC",
                    "body": "Rain hammers the neon alley outside Dock 4.",
                }
            ],
        }
    )
    assert "[PUBLIC]" in header
    assert "Rain hammers the neon alley outside Dock 4." in header


def test_wave2_h6_http_errors_offline_dice_snap_coords_and_llm_guard(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    sid = f"w2-h6-{uuid.uuid4().hex[:8]}"
    uid = "ai-player-h6"
    monkeypatch.setenv("SATYA_SIGNALING_URL", "offline")

    try:
        set_player_session_context(
            sid,
            uid,
            {
                "roster": [{"user_id": uid, "display_name": "Mateo"}],
                "active_map": {
                    "map_id": "m-h6",
                    "title": "Alley",
                    "grid_width": 12,
                    "grid_height": 12,
                    "active_floor": 0,
                    "tokens": [{"entity_id": uid, "label": "Mateo", "x": 2, "y": 2}],
                },
            },
        )

        # 1. HTTP 4xx/5xx error surfaces explicitly instead of silent success
        monkeypatch.setattr(
            "satya_ml.ai_player.tools._http_post_json_detailed",
            lambda *_a, **_kw: (None, 403, "Forbidden action"),
        )
        door_err = toggle_player_door.invoke(
            {"cell_x": 3, "cell_y": 2, "side": "NORTH", "session_id": sid, "player_user_id": uid}
        )
        assert door_err["status_code"] == 403
        assert "Forbidden action" in door_err["error"]

        chat_err = send_player_chat.invoke(
            {"message": "Hello", "session_id": sid, "player_user_id": uid}
        )
        assert chat_err["status_code"] == 403

        move_err = move_player_map_token.invoke(
            {"target_x": 4.5, "target_y": 3.5, "session_id": sid, "player_user_id": uid}
        )
        assert move_err["status_code"] == 403

        # 2. Snapped coordinates returned by server are reported cleanly
        monkeypatch.setattr(
            "satya_ml.ai_player.tools._http_post_json_detailed",
            lambda *_a, **_kw: (
                {
                    "map_id": "m-h6",
                    "title": "Alley",
                    "grid_width": 12,
                    "grid_height": 12,
                    "active_floor": 0,
                    "tokens": [
                        {
                            "entity_id": uid,
                            "label": "Mateo",
                            "x": 4,
                            "y": 3,
                            "pos_x_cm": 450,
                            "pos_y_cm": 350,
                        }
                    ],
                },
                200,
                None,
            ),
        )
        snapped = move_player_map_token.invoke(
            {"target_x": 9.5, "target_y": 9.5, "session_id": sid, "player_user_id": uid}
        )
        assert snapped["status"] == "moved"
        assert snapped["x_m"] == 4.5
        assert snapped["y_m"] == 3.5

        # 3. Offline dice use _PLAYER_RNG rather than repeating a constant [3, 7] pair
        monkeypatch.setattr(
            "satya_ml.ai_player.tools._http_post_json_detailed",
            lambda *_a, **_kw: (None, None, None),
        )
        dice_pairs: list[tuple[int, ...]] = []
        for _ in range(10):
            r = roll_player_d20_action.invoke(
                {
                    "check_category": "DEX Stealth",
                    "attribute_tn": 14,
                    "skill_rank": 3,
                    "session_id": sid,
                    "player_user_id": uid,
                }
            )
            rolls = tuple(int(d) for pair in r.get("rolled_pairs", []) for d in pair)
            assert len(rolls) >= 2
            assert all(1 <= d <= 20 for d in rolls)
            dice_pairs.append(rolls)
        assert len(set(dice_pairs)) > 1

        # 4. Failing chat_model.invoke is caught cleanly inside player_decision_node
        class _ExplodingModel:
            def bind_tools(self, _tools: list[Any]) -> _ExplodingModel:
                return self

            def invoke(self, _messages: list[BaseMessage]) -> AIMessage:
                raise RuntimeError("boom-llm-offline")

        pgraph = build_ai_player_graph(
            chat_model=_ExplodingModel(),
            tools=[],
            checkpointer=MemorySaver(),
        )
        out = pgraph.invoke(
            {
                "session_id": sid,
                "player_user_id": uid,
                "messages": [],
                "trigger_event": {"content": "Move forward."},
            },
            config={"configurable": {"thread_id": f"{sid}:{uid}"}},
        )
        assert out.get("final_reply")
    finally:
        reset_player_session_context(sid)
        reset_player_motivation_ledgers(sid)


def test_wave2_h7_canonical_sheet_extraction_and_no_char_theft(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    sid = f"w2-h7-{uuid.uuid4().hex[:8]}"
    uid = "ai-player-unclaimed"
    monkeypatch.setenv("SATYA_SIGNALING_URL", "offline")

    try:
        clean = sanitize_player_visible_context(
            {
                "session_id": sid,
                "bound_characters": [
                    {
                        "owner_user_id": "human-player-other",
                        "character_id": "pc-stolen",
                        "character_name": "Stolen Character",
                    }
                ],
            },
            uid,
        )
        assert clean["character_sheet"]["owner_user_id"] == uid
        assert clean["character_sheet"].get("character_name") != "Stolen Character"

        canonical_sheet = {
            "character_id": "pc-canon-h7",
            "owner_user_id": uid,
            "character_name": "Nadia Lin",
            "characteristics": {"ins": 16, "itu": 15, "spd": 14, "dis": 14},
            "psyche": {
                "self_vs_selfless": 15,
                "cautious_vs_bold": 12,
                "conformist_vs_eccentric": 11,
                "empathetic_vs_aloof": 16,
            },
            "skills": {"empathy_social_perception": 4},
        }
        set_player_session_context(
            sid,
            uid,
            {
                "roster": [{"user_id": uid, "display_name": "Nadia"}],
                "character_sheet": canonical_sheet,
                "public_npcs": [{"npc_id": "npc-clerk", "name": "Desk Clerk"}],
            },
        )
        ledger = ensure_or_sync_motivation_ledger(sid, uid, canonical_sheet, "")
        assert isinstance(ledger.get("psyche_scores"), dict)
        emp = perform_player_empathy_read.invoke(
            {
                "target_npc_name": "Desk Clerk",
                "net_successes": 0,
                "session_id": sid,
                "player_user_id": uid,
            }
        )
        assert "error" not in emp
    finally:
        reset_player_session_context(sid)
        reset_player_motivation_ledgers(sid)


def test_wave2_h8_empathy_visibility_and_map_sanitization(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    sid = f"w2-h8-{uuid.uuid4().hex[:8]}"
    uid = "ai-player-h8"
    monkeypatch.setenv("SATYA_SIGNALING_URL", "offline")

    raw_map = {
        "map_id": "m-h8",
        "title": "Pier 9",
        "grid_width": 12,
        "grid_height": 10,
        "active_floor": 0,
        "notes": "SECRET GM TRAP DOOR AT (4,4)",
        "tokens": [
            {"entity_id": uid, "label": "Mateo", "x": 1, "y": 1, "visibility": "PUBLIC"},
            {"entity_id": "npc-dock", "label": "Dockhand", "x": 3, "y": 2, "visibility": "PUBLIC"},
            {
                "entity_id": "secret-agent",
                "label": "Hidden Sniper",
                "x": 9,
                "y": 8,
                "visibility": "GM_ONLY",
            },
        ],
    }
    clean_map = sanitize_map_dict_for_player(raw_map)
    assert clean_map is not None
    assert "notes" not in clean_map
    vis_ids = {t["entity_id"] for t in clean_map["tokens"]}
    assert vis_ids == {uid, "npc-dock"}

    try:
        set_player_session_context(
            sid,
            uid,
            {
                "roster": [{"user_id": uid, "display_name": "Mateo"}],
                "active_map": raw_map,
                "public_npcs": [{"npc_id": "npc-dock", "name": "Dockhand"}],
            },
        )
        blocked = perform_player_empathy_read.invoke(
            {
                "target_npc_name": "secret-agent",
                "net_successes": 3,
                "session_id": sid,
                "player_user_id": uid,
            }
        )
        assert "not visibly present" in blocked.get("error", "")

        allowed = perform_player_empathy_read.invoke(
            {
                "target_npc_name": "Dockhand",
                "net_successes": 0,
                "session_id": sid,
                "player_user_id": uid,
            }
        )
        assert "error" not in allowed
    finally:
        reset_player_session_context(sid)
        reset_player_motivation_ledgers(sid)


def test_wave2_h9_satya_py_presets_override_and_python_map_sync(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    import satya_py

    with pytest.raises(ValueError):
        satya_py.generate_procedural_tactical_sector_json(
            "m-bad",
            "nonexistent_biome_xyz",
            None,
            None,
            None,
            42,
        )

    auto_json = satya_py.generate_procedural_tactical_sector_json(
        "m-auto",
        "hospital",
        "Auto Clinic",
        None,
        None,
        19,
    )
    auto_obj = json.loads(auto_json)
    assert int(auto_obj["grid_width"]) >= 16
    assert int(auto_obj["grid_height"]) >= 12

    auto_obj["tokens"] = [
        {
            "entity_id": "tok-custom",
            "label": "Scout Drone",
            "x": 2,
            "y": 2,
            "pos_x_cm": 250,
            "pos_y_cm": 250,
            "visibility": "PUBLIC",
        }
    ]
    auto_obj["token_visuals"] = [
        {
            "entity_id": "tok-custom",
            "model_preset": "custom_drone_mk2",
            "color_hex": "#f97316",
            "facing_deg": 225,
            "floor_index": 0,
            "pos_x_cm": 250,
            "pos_y_cm": 250,
        }
    ]

    cont_json = satya_py.continue_procedural_tactical_map_json(
        json.dumps(auto_obj),
        "EAST",
        55,
        "warehouse",
    )
    cont_obj = json.loads(cont_json)
    assert any(
        str(s.get("theme_preset") or s.get("preset") or "") == "warehouse"
        for s in (cont_obj.get("world_sectors") or [])
        if isinstance(s, dict)
    )
    custom_vis = next(
        v for v in (cont_obj.get("token_visuals") or []) if v.get("entity_id") == "tok-custom"
    )
    assert custom_vis["model_preset"] == "custom_drone_mk2"
    assert custom_vis["color_hex"] == "#f97316"
    assert custom_vis["facing_deg"] == 225

    assert infer_sector_preset("downtown mansion") == "luxury_mansion"
    assert infer_sector_preset("subterranean bunker") == "derelict_corridor"

    probe_map = {
        "map_id": "m-prop-probe",
        "title": "Prop Probe",
        "grid_width": 10,
        "grid_height": 10,
        "wall_edges": [],
        "props": [
            {
                "id": "p-creek",
                "kind": "CREEK_WATER",
                "center_x_cm": 250,
                "center_y_cm": 250,
                "span_w_cm": 120,
                "span_h_cm": 120,
                "rotation_deg": 0,
            },
            {
                "id": "p-catwalk",
                "kind": "ELEVATED_CATWALK",
                "center_x_cm": 450,
                "center_y_cm": 250,
                "span_w_cm": 120,
                "span_h_cm": 120,
                "rotation_deg": 0,
            },
            {
                "id": "p-hedge",
                "kind": "HEDGE",
                "center_x_cm": 500,
                "center_y_cm": 650,
                "span_w_cm": 120,
                "span_h_cm": 120,
                "rotation_deg": 0,
                "cover_height_cm": 140,
            },
        ],
    }
    assert satya_py.maps.check_token_collision(probe_map, 2.5, 2.5) is False
    assert satya_py.maps.check_token_collision(probe_map, 4.5, 2.5) is False
    los_hedge = satya_py.maps.evaluate_los_and_cover(probe_map, 2.5, 6.5, 7.5, 6.5)
    assert str(los_hedge.get("cover_grade", "")).upper() == "CONCEALMENT"

    sid = f"w2-h9-{uuid.uuid4().hex[:8]}"
    monkeypatch.setenv("SATYA_SIGNALING_URL", "offline")
    try:
        generated = generate_tactical_map.invoke(
            {
                "session_id": sid,
                "theme": "hospital",
                "seed": 77,
                "initial_tokens": [
                    {"token_id": "tok-mv", "label": "Mateo", "grid_x": 3, "grid_y": 3}
                ],
            }
        )
        assert generated.get("floors")
        register_spatial_clue_pin.invoke(
            {
                "session_id": sid,
                "pin_id": "pin-1",
                "channel": "SIGHT",
                "pos_x_m": 4.0,
                "pos_y_m": 4.0,
                "radius_m": 2.0,
                "whisper_text": "Footprints.",
            }
        )
        expected_f1_elev = next(
            int(fl.get("elevation_cm", 300))
            for fl in generated["floors"]
            if int(fl.get("floor_index", 0)) == 1
        )
        moved = move_map_token.invoke(
            {
                "session_id": sid,
                "token_id": "tok-mv",
                "to_x_cm": 350,
                "to_y_cm": 350,
                "to_floor": 1,
            }
        )
        tok = next(t for t in moved["tokens"] if t["token_id"] == "tok-mv")
        assert tok["floor_index"] == 1
        assert tok["elevation_cm"] == expected_f1_elev
        vis = next(v for v in moved["token_visuals"] if v["entity_id"] == "tok-mv")
        assert vis["floor_index"] == 1
        assert vis["elevation_cm"] == expected_f1_elev

        clear_session_map_caches(sid)
        assert get_session_map(sid) is None
    finally:
        reset_session_npcs(sid)


def test_wave2_h11_h12_ai_gm_narration_mode_reset_and_isolation(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    sid_a = f"w2-h12-a-{uuid.uuid4().hex[:8]}"
    sid_b = f"w2-h12-b-{uuid.uuid4().hex[:8]}"
    posted_bodies: list[str] = []

    class _FakeResp:
        status_code = 200

        def json(self) -> dict[str, Any]:
            return {"message_id": "msg-ok"}

    class _FakeHttpxClient:
        def __init__(self, **_kwargs: Any) -> None:
            pass

        def __enter__(self) -> _FakeHttpxClient:
            return self

        def __exit__(self, *_args: Any) -> None:
            pass

        def get(self, *_args: Any, **_kwargs: Any) -> Any:
            raise RuntimeError("offline")

        def post(self, url: str, **kwargs: Any) -> _FakeResp:
            if url.endswith("/chat"):
                body = (kwargs.get("json") or {}).get("body")
                if body:
                    posted_bodies.append(str(body))
            return _FakeResp()

    monkeypatch.setenv("SATYA_SIGNALING_URL", "http://127.0.0.1:3999")
    monkeypatch.setattr("satya_ml.ai_gm.graph.fetch_session_context_from_rust", lambda _s: None)
    monkeypatch.setattr("satya_ml.ai_gm.graph.report_trajectory_to_rust", lambda *_a: None)
    monkeypatch.setattr("satya_ml.ai_gm.graph.httpx.Client", _FakeHttpxClient)

    @tool
    def lookup_npc_profile(npc_id: str, session_id: str = "default") -> dict[str, Any]:
        """Stub lookup tool (not narrate_to_session)."""
        return {"npc_id": npc_id, "session_id": session_id}

    @tool
    def narrate_to_session(session_id: str = "default", narrative: str = "") -> dict[str, Any]:
        """Stub narrate_to_session tool."""
        return {"status": "ok", "session_id": session_id, "narrative": narrative}

    class _StepModel:
        def __init__(self, tool_name: str, tool_args: dict[str, Any], reply: str) -> None:
            self.tool_name = tool_name
            self.tool_args = tool_args
            self.reply = reply
            self.seen_system_prompts: list[str] = []

        def bind_tools(self, _tools: list[Any]) -> _StepModel:
            return self

        def invoke(self, messages: list[BaseMessage]) -> AIMessage:
            if messages and getattr(messages[0], "type", "") == "system":
                self.seen_system_prompts.append(str(messages[0].content))
            if getattr(messages[-1], "type", "") == "tool":
                return AIMessage(content=self.reply)
            return AIMessage(
                content="",
                tool_calls=[
                    {
                        "name": self.tool_name,
                        "args": self.tool_args,
                        "id": f"call-{self.tool_name}-{len(self.seen_system_prompts)}",
                    }
                ],
            )

    try:
        # H11: lookup_npc_profile does NOT set already_narrated, so publish_gm_response posts once
        non_narr_model = _StepModel("lookup_npc_profile", {"npc_id": "vance"}, "GM fallback text")
        g1 = build_ai_gm_graph(
            chat_model=non_narr_model,
            tools=[lookup_npc_profile, narrate_to_session],
            checkpointer=MemorySaver(),
        )
        g1.invoke(
            {
                "session_id": sid_a,
                "operating_mode": "autonomous",
                "trigger_event": {"content": "Who is Vance?", "actor": {"display_name": "P"}},
            },
            config={"configurable": {"thread_id": f"{sid_a}-t1"}},
        )
        assert posted_bodies == ["GM fallback text"]

        # H11: narrate_to_session sets already_narrated=True, suppressing duplicate post
        posted_bodies.clear()
        narr_model = _StepModel("narrate_to_session", {"narrative": "Direct"}, "Duplicate summary")
        g2 = build_ai_gm_graph(
            chat_model=narr_model,
            tools=[lookup_npc_profile, narrate_to_session],
            checkpointer=MemorySaver(),
        )
        g2.invoke(
            {
                "session_id": sid_a,
                "operating_mode": "autonomous",
                "trigger_event": {"content": "Open scene.", "actor": {"display_name": "P"}},
            },
            config={"configurable": {"thread_id": f"{sid_a}-t2"}},
        )
        assert posted_bodies == []

        # H12: SystemMessage refreshes in place when mode switches across turns on same thread
        saver = MemorySaver()
        mode_model = _StepModel("lookup_npc_profile", {"npc_id": "vance"}, "Ok")
        g3 = build_ai_gm_graph(
            chat_model=mode_model,
            tools=[lookup_npc_profile],
            checkpointer=saver,
        )
        g3.invoke(
            {
                "session_id": sid_b,
                "operating_mode": "autonomous",
                "trigger_event": {"content": "Turn 1", "actor": {"display_name": "P"}},
            },
            config={"configurable": {"thread_id": sid_b}},
        )
        g3.invoke(
            {
                "session_id": sid_b,
                "operating_mode": "co_pilot",
                "human_gm_user_id": "gm-human",
                "trigger_event": {"content": "Turn 2", "actor": {"display_name": "P"}},
            },
            config={"configurable": {"thread_id": sid_b}},
        )
        assert mode_model.seen_system_prompts[0] == AUTONOMOUS_SYSTEM_PROMPT
        assert mode_model.seen_system_prompts[-1] == CO_PILOT_SYSTEM_PROMPT

        # H12: clearing human_gm_user_id pops _SESSION_HUMAN_GMS[sid]
        set_session_operating_mode(sid_b, "co_pilot", "gm-human")
        assert get_session_human_gm(sid_b) == "gm-human"
        set_session_operating_mode(sid_b, "autonomous", None)
        assert get_session_human_gm(sid_b) is None

        # H12: per-session Scenario Director reset isolates sibling sessions
        set_session_scenario_director(sid_a, {"campaign_premise": "Session A Premise"})
        set_session_scenario_director(sid_b, {"campaign_premise": "Session B Premise"})
        reset_session_scenario_director(sid_b)
        assert get_session_scenario_director(sid_a)["campaign_premise"] == "Session A Premise"

        # H12: get_session_npcs non-destructively preserves local NPC state without gm_character
        monkeypatch.setattr("satya_ml.ai_gm.tools._signaling_base_url", lambda: "")
        npcs_local = get_session_npcs(sid_a)
        first_id = next(iter(npcs_local))
        npcs_local[first_id]["disposition"] = "ALLIED_SECRET"
        npcs_local[first_id]["secrets"] = ["Local Secret"]
        monkeypatch.setattr("satya_ml.ai_gm.tools._signaling_base_url", lambda: "http://127.0.0.1:3999")
        monkeypatch.setattr(
            "satya_ml.ai_gm.tools._http_get_json",
            lambda *_a, **_kw: {"npcs": [{"character_id": first_id, "name": "Updated Name"}]},
        )
        merged = get_session_npcs(sid_a)
        assert merged[first_id]["disposition"] == "ALLIED_SECRET"
        assert merged[first_id]["secrets"] == ["Local Secret"]
    finally:
        reset_session_npcs(sid_a)
        reset_session_npcs(sid_b)
        reset_session_scenario_director(sid_a)
        reset_session_scenario_director(sid_b)


def test_wave2_h13_player_multifloor_and_door_tool(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    sid = f"w2-h13-{uuid.uuid4().hex[:8]}"
    uid = "ai-player-h13"
    monkeypatch.setenv("SATYA_SIGNALING_URL", "http://127.0.0.1:3999")

    multi_map = {
        "map_id": "m-h13",
        "title": "Research Wing",
        "grid_width": 12,
        "grid_height": 12,
        "active_floor": 0,
        "floors": [
            {"floor_index": 0, "label": "1F", "wall_edges": [], "props": [], "surfaces": []},
            {
                "floor_index": 1,
                "label": "2F",
                "wall_edges": [{"x": 3, "y": 4, "side": "NORTH", "kind": "DOOR_CLOSED"}],
                "props": [{"kind": "STAIRWELL", "center_x_cm": 500, "center_y_cm": 500}],
                "surfaces": [],
            },
        ],
        "tokens": [
            {"entity_id": uid, "label": "Mateo", "x": 4, "y": 5, "visibility": "PUBLIC"},
            {"entity_id": "npc-1f", "label": "Guard", "x": 8, "y": 8, "visibility": "PUBLIC"},
        ],
        "token_visuals": [
            {"entity_id": uid, "floor_index": 1, "pos_x_cm": 450, "pos_y_cm": 550},
            {"entity_id": "npc-1f", "floor_index": 0, "pos_x_cm": 850, "pos_y_cm": 850},
        ],
    }

    try:
        summary_str = summarize_public_map_for_player(multi_map, uid, None)
        assert "MyFloor 1" in summary_str
        assert "(4.5m, 5.5m) on Floor 1" in summary_str
        assert "Floor 0 — different floor" in summary_str

        set_player_session_context(
            sid,
            uid,
            {"roster": [{"user_id": uid, "display_name": "Mateo"}], "active_map": multi_map},
        )
        surr = inspect_surroundings.invoke({"session_id": sid, "player_user_id": uid})
        assert surr["observer_m"] == {"x_m": 4.5, "y_m": 5.5, "floor_index": 1}
        assert len(surr["vertical_portals"]) >= 1
        assert len(surr["doors"]) >= 1

        recorded: list[tuple[str, dict[str, Any]]] = []

        def _fake_post(url: str, payload: dict[str, Any], **_kw: Any) -> tuple[dict[str, Any] | None, int | None, str | None]:
            recorded.append((url, payload))
            return ({**multi_map, "event_type": "DOOR_TOGGLED"}, 200, None)

        monkeypatch.setattr("satya_ml.ai_player.tools._http_post_json_detailed", _fake_post)
        res = toggle_player_door.invoke(
            {"cell_x": 3, "cell_y": 4, "side": "north", "session_id": sid, "player_user_id": uid}
        )
        assert res["status"] == "toggled"
        assert recorded and recorded[0][0].endswith(f"/api/player-ai/sessions/{sid}/door")
        assert recorded[0][1] == {
            "user_id": uid,
            "cell_x": 3,
            "cell_y": 4,
            "side": "NORTH",
        }
    finally:
        reset_player_session_context(sid)
        reset_player_motivation_ledgers(sid)
