# `satya-py` Agent Coding & Architecture Rules

When reading, adding, or refactoring code in `crates/satya-py` (or Python consumers in `src/satya_ml/` and `notebooks/`), activate the [`satya-rust-workspace`](../../.agents/skills/satya-rust-workspace/SKILL.md) and [`satya-rust-engine`](../../.agents/skills/satya-rust-engine/SKILL.md) skills and follow these architectural rules (full guide in [`docs/technical/rust-workspace-steering.md`](../../docs/technical/rust-workspace-steering.md) and [`docs/technical/rust-engine-steering.md`](../../docs/technical/rust-engine-steering.md)):

1. **Thin PyO3 Wrapper Over `satya-engine` & `satya-schema` (Zero Duplicate Math)**:
   - `satya-py` is a thin PyO3 (`abi3-py312`) binding layer exposing `satya-engine` and `satya-schema` to Python (`satya_ml`) and Marimo notebooks (`notebooks/`).
   - Never implement probability distributions, `2d20` cascade resolution, Tactical Pool math, character creation auditing, or Avro `CRC-64-AVRO` fingerprinting inside `crates/satya-py`; always delegate to `satya-engine` and `satya-schema` functions and convert results via `From` / `Into`.

2. **Pragmatic File Size Ceiling (`<= 45 KB`) & Companion Submodules**:
   - Aim to keep `.rs` files in `crates/satya-py/src/` under `45 KB` (`< 46,080` bytes) by splitting large PyO3 class/function groups (e.g. dice/cascade wrappers, combat/character wrappers, schema/rules wrappers) into companion modules with flat `pub use` re-exports registered on the root `#[pymodule] fn satya_py`.

3. **Canonical Sign Conventions & Parameter Naming**:
   - Preserve exact parameter names and sign conventions from `satya-engine` (`skill_level`, `control_c`, `internal_mod` / `delta_int`, `known_tool_mod` / `delta_tool`, `external_mod` / `delta_ext`) and encode physical/time units in identifiers (`_ms`, `_us`, `_deg`, `_m`, `_kg`, `_grains`, `_joules`).

4. **High-Signal Docstrings (No `#![deny(missing_docs)]`)**:
   - Do not enable `#![deny(missing_docs)]`. Provide concise `//!` module headers and `///` docstrings on exported `#[pyclass]` and `#[pyfunction]` items so Python `help(satya_py)` and IDE hover tooltips remain informative without boilerplate on internal plumbing.
