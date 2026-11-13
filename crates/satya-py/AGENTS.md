# `satya-py` Agent Coding & Architecture Rules

Activate [`satya-rust-workspace`](../../.agents/skills/satya-rust-workspace/SKILL.md) and [`satya-rust-engine`](../../.agents/skills/satya-rust-engine/SKILL.md) when modifying `crates/satya-py` or Python consumers in `src/satya_ml/` and `notebooks/`.

1. **Thin PyO3 (`abi3-py312`) Bridge — Zero Duplicate Math or Spatial Geometry**:
   - Module layout: `src/lib.rs` (Cascade, Triad, Rules Book, Avro/GDPR) and `src/tactical_map_py.rs` (`check_tactical_token_collision`, `resolve_tactical_token_destination_cm`, `evaluate_tactical_los_and_cover`, `inspect_tactical_map_point`, `generate_procedural_tactical_sector_json`).
   - Always delegate dice/cascade math, tactical map collision/LoS/hover geometry, and Avro `CRC-64-AVRO` checks to `satya-engine` and `satya-schema`. Never mutate locked `v1` structs to accommodate flexible Python dicts; normalize inputs inside `tactical_map_py.rs`.
2. **Windows Offline Build, `PYO3_PYTHON`, & `src\satya_py.pyd` Sync**:
   - Set `$env:PYO3_PYTHON = 'C:\Users\hapah\AppData\Roaming\uv\python\cpython-3.14-windows-x86_64-none\python.exe'`.
   - Run `cargo clippy --offline --no-default-features -p satya-py --all-targets -- -D warnings` and `cargo test --offline --no-default-features -p satya-py` (no Python link errors in test binaries), then `cargo build --offline -p satya-py` and copy `target\debug\satya_py.dll` to `src\satya_py.pyd`.
3. **AI GM Consumers**: Follow [`src/satya_ml/ai_gm/AGENTS.md`](../../src/satya_ml/ai_gm/AGENTS.md) and [`docs/technical/ai-gm-agent-design.md`](../../docs/technical/ai-gm-agent-design.md).
