---
name: satya-python-ai
description: >-
  Architecture, 5-step LangGraph @tool registration checklist, PyO3 satya_py bridge
  conventions, session-state reset & seeding order, hybrid PyLance RAG visibility
  wall, and fast pyrefly + pytest-xdist verification for src/satya_ml/ (ai_gm,
  ai_player, rag) and crates/satya-py. Activate this skill whenever creating,
  modifying, or testing Python AI GM, AI Player, RAG, or PyO3 wrapper code.
---

# Satya Python AI GM, AI Player & Hybrid RAG Skill (`satya-python-ai`)

## 1. Thin Python Orchestration Over Hybrid `satya_py` Package (`src/satya_py/`)
- Keep deterministic TTRPG math, 9-axis Psyche (`EGO`, `SLF`, `OPN`, `BLG`, `EXT`, `CNF`, `AGG`, `TRS`, `CPS`), social combat, spatial LoS/cover, Ch. 7/11/18/20–24 technical gear/EW/hazards, and *Awakened* substrate/arc/Part V mechanics in Rust exposed via `satya_py._native` (`crates/satya-py`, stubbed in `src/satya_py/_native.pyi`) and its typed domain submodules (`satya_py.{dice,combat,rules,maps,campaign,psyche,awakened,tech,schema}`).
- Do **not** write parallel pure-Python mechanics fallbacks when adding or refactoring features; `pixi run test-py` depends on `build-py` (`src/satya_py/_native.pyd`), so `satya_py` is always available in tests and runtime.
- Organize `.py` files by cohesive domain responsibility (`social_and_clocks.py`, `npc_psyche_combat.py`, `audit_and_first_contact.py`, `awakened_systems.py`, `tech_and_tradecraft.py`, `rag/{corpus_builder,hybrid_index}.py`) rather than accumulating unrelated tools in `tools.py`, `graph.py`, or `maps.py` — split by concern, not line count.

## 2. 5-Step Checklist When Adding or Renaming an AI GM `@tool`
Every new LangGraph AI GM tool must update all 5 sites together:
1. **Define `@tool` & Session Store**: Implement in a focused domain module under `src/satya_ml/ai_gm/` using `session_id: str = "default"`.
2. **Session Reset Hook**: Clear any module `_SESSION_*` dict inside `reset_social_and_world_state(session_id)` (`social_and_clocks.py`) so parallel/multi-test sessions never leak state.
3. **Tool Registry**: Import and append the tool to `get_builtin_gm_tools()` in `src/satya_ml/ai_gm/tools.py`, and add its exact name to `CORE_GM_TOOL_NAMES` or the appropriate domain pack in `DEFERRED_TOOL_PACKS` (`src/satya_ml/ai_gm/tool_registry.py`).
4. **Telemetry Tier Classification**: Register the exact tool name in `_classify_tool_tier()` in `src/satya_ml/ai_gm/graph.py` (`TIER_1_CORE_DICE`, `TIER_2_RULESET_ENGINE`, or `TIER_3_OPTIONAL_FLAVOR`).
5. **Package Re-Export**: Export in `src/satya_ml/ai_gm/__init__.py`.

## 3. Session Seeding Order & Cyclic-Import Pitfalls
- **Lazy Import Verification**: `graph.py` ↔ `tools.py` ↔ `scenario_director.py` ↔ `party_coordinator.py` have runtime import cycles. Whenever adding a helper call (e.g. `get_active_campaign_pack_name()`) inside one of those modules, verify the symbol is imported in scope by running `pixi run pyrefly check <edited_files>` before launching full pytest.
- **`world_ledger` Economic Tier Seeding Order**: Tools calling `get_session_world_ledger(session_id)` lazily seed a default `MIDDLE_CLASS` ledger if none exists. In tests or scenarios requiring `LOW_INCOME` / `DESTITUTE` / `WEALTHY` baseline strain, call `generate_tactical_map(..., economic_tier=...)` (or seed the world ledger explicitly) **before** calling telemetry/social/grid tools.
- **Fast Python / Lore-Only Pytest (Skip `build-py`)**: `pixi run test-py` forces `depends-on = ["build-py"]` (`--release` Rust compile). Whenever `.rs` files and `docs/ttrpg/*.md` were not modified, run targeted or parallel pytest directly with `PYTHONPATH=src` instead: `pixi run python -m pytest tests/<relevant_test>.py` (or `pixi run python -m pytest -n auto --dist loadfile tests`), avoiding release Rust rebuilds and `src/satya_py/_native.pyd` Windows file locks.
- **Offline Signaling in Tests**: `tests/conftest.py` short-circuits offline signaling HTTP retries so `pixi run test-py` (`pytest -n auto --dist loadfile tests`) finishes in ~11s across workers. Keep per-test `session_id` strings unique within a test file.
- **Windows Locked `src/satya_py/_native.pyd` Fallback (`os error 32`)**: If a live server (e.g., `satya_ml.ai_gm.server`) holds `src/satya_py/_native.pyd` open on Windows, `pixi run build-py` / `pixi run test-py` will fail updating `src/satya_py/_native.pyd`. Never kill the user's running server; instead compile with `cargo build --offline --release -p satya-py`, restart or isolate the lock before replacing `src/satya_py/_native.pyd` in place. Also scope quick `pixi run pyrefly check <edited_files>` runs to touched files first so pre-existing unrelated repo errors don't mask regressions.

## 4. Hybrid PyLance RAG & Player/GM Visibility Wall (`src/satya_ml/rag/`)
- Corpus lives at `data/lance/satya-knowledge-rag.lance` (`pixi run build-rag`), indexing `ttrpg_rules` (H1/H2/H3 + 12-row header-preserving table slices), `rust_engine` symbol cards, and `campaign_lore` (`PUBLIC`, `ARC2_SATYANG`, `GM_CONSPIRACY_SECRET`).
- **Strict Visibility Invariant**: Any AI Player tool (`src/satya_ml/ai_player/motivation_ledger.py` `lookup_public_lore_and_npcs`) **must** query `SatyaHybridRagIndex.search(..., caller_role="player")`, which enforces `visibility == "PUBLIC"` so `GM_CONSPIRACY_SECRET` / `ARC2_SATYANG` chapters (`04-chronicle-and-gm/`, `01-opening/prologue.md`, `03-minds-and-factions/satyang_and_four_pillars.md`) never leak to player agents.
