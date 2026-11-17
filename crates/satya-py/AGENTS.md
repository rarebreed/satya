# `satya-py` Agent Coding & Architecture Rules

Activate [`satya-rust-workspace`](../../.agents/skills/satya-rust-workspace/SKILL.md), [`satya-rust-engine`](../../.agents/skills/satya-rust-engine/SKILL.md), and [`satya-python-ai`](../../.agents/skills/satya-python-ai/SKILL.md) when modifying `crates/satya-py` or Python consumers in `src/satya_ml/` and `notebooks/`.

1. **Thin PyO3 (`abi3-py312`) Bridge — Zero Duplicate Math or Spatial Geometry**:
   - **Module Responsibility Map** (exact exported signatures live in [`src/satya_py.pyi`](../../src/satya_py.pyi)):
     - `src/lib.rs`: Module root plus `2d20` Cascade/Triad evaluation, Rules Book lookup, Avro framing, and GDPR helpers.
     - `src/tactical_map_py.rs`: Spatial collision/destination resolution, LoS/cover evaluation, sector generation/continuation, floor switches, and passive movement perception.
     - `src/hooks_and_npc_py.rs`: Party motivational hook compilation, lifepath anchors, and pack-aware NPC profile generation.
     - `src/psyche_social_py.rs`: 9-axis Psyche empathy/dissonance/conviction/drift, social combat, morale triggers, and `SessionWorldLedger` wrappers.
     - `src/awakened_systems_py.rs`: Split-brain synthetic substrate state, L0/L1 cage/hunter AI evaluation, conspiracy ledgers, polycrisis grids, tradecraft, and dissenter/proxy hooks.
   - Always delegate dice/cascade math, tactical map collision/LoS/hover geometry, passive perception checks, motivational hook compilation, social combat/Psyche mechanics, Awakened substrate/conspiracy/polycrisis/tradecraft mechanics, world clocks, and Avro `CRC-64-AVRO` checks to `satya-engine` and `satya-schema`. Never mutate locked `v1` structs to accommodate flexible Python dicts; normalize inputs inside `tactical_map_py.rs`, `hooks_and_npc_py.rs`, `psyche_social_py.rs`, and `awakened_systems_py.rs`.
2. **Pixi Build, Clippy/Test & `src/satya_py.pyi` Stub Sync**:
   - Always run commands via `pixi run ...` (never call `.pixi/envs/default/*.exe` directly or hardcode external `uv` Python paths).
   - Run `pixi run cargo clippy --offline --no-default-features -p satya-py --all-targets -- -D warnings` and `pixi run cargo test --offline --no-default-features -p satya-py` (`--no-default-features` disables `auto-initialize` so Rust test binaries link cleanly), and run `pixi run build-py` to compile release `satya_py` and update the gitignored `src/satya_py.pyd` binary.
   - `src/satya_py.pyi` is the tracked PEP 484 stub file for `satya_py`: keep it in sync whenever `#[pyclass]` or `#[pyfunction]` exports change (verified by `test_satya_py_stub_matches_compiled_extension_exports` and `pixi run pyrefly check src/satya_py.pyi`).
3. **AI GM Consumers**: Follow [`src/satya_ml/ai_gm/AGENTS.md`](../../src/satya_ml/ai_gm/AGENTS.md) and [`docs/technical/ai-gm-agent-design.md`](../../docs/technical/ai-gm-agent-design.md).
