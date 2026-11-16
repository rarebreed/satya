# Satya Workspace Agent Coding & Architecture Rules

When working on any Rust crate, Python module, or documentation file in this repository, activate the corresponding skill in `.agents/skills/` and follow the universal rules in [`.agents/skills/satya-rust-workspace/SKILL.md`](.agents/skills/satya-rust-workspace/SKILL.md):

- **All Rust Crates (`crates/*`) & Python Modules (`src/satya_ml/*`, `tests/*`)**: Skill [`satya-rust-workspace`](.agents/skills/satya-rust-workspace/SKILL.md)
  - Target `~300–750` lines (`<= 45 KB`) per `.rs`, `.py`, `.css`, and `.md` file via focused companion modules with flat `pub use` re-exports in the parent module (avoid duplicate glob re-exports in `lib.rs` / `mod.rs`) so every file fits inside a single `read_file` call.
  - Zero duplicate domain types, formulas, or geometry helpers across crates; prefer declarative macros (`define_wire_enum!`, `define_coded_enum!`, `bind_window_callbacks!`) and `const` tables.
  - High-signal `//!` and `///` docstrings (no `#![deny(missing_docs)]`) + unit suffixes (`_ms`, `_us`, `_deg`, `_m`, `_cm`, `_kg`).
  - **Strict `LF` (`\n`) line endings**: Save every file with `\n` endings — never `CRLF` (`\r\n`) — and never run parallel `edit_file` calls on the same file.
  - **Pixi Environment Execution**: Always run build, lint, Cargo, and Python commands through `pixi run <task-or-cmd>` (e.g., `pixi run test-py`, `pixi run build-py`, `pixi run build-rag`, `pixi run cargo ...`). Never invoke binaries directly from `.pixi/envs/default/*.exe` or hardcoded `uv` interpreter paths.
- **`crates/satya-schema` & `schemas/v*/`**: [`crates/satya-schema/AGENTS.md`](crates/satya-schema/AGENTS.md)
- **`crates/satya-engine` & `docs/ttrpg/`**: Skill [`satya-rust-engine`](.agents/skills/satya-rust-engine/SKILL.md) · [`crates/satya-engine/AGENTS.md`](crates/satya-engine/AGENTS.md)
- **`crates/satya-storage` (Apache Iggy WAL & Avro Batch Sink)**: [`crates/satya-storage/AGENTS.md`](crates/satya-storage/AGENTS.md)
- **`crates/satya-signaling` (Axum HTTP/WS Server, Auth, GM Vault & AI GM Bridge)**: [`crates/satya-signaling/AGENTS.md`](crates/satya-signaling/AGENTS.md)
- **`crates/satya-web` (Leptos SSR + WebAssembly + WebGPU Client)**: Skill [`satya-rust-web`](.agents/skills/satya-rust-web/SKILL.md) · [`crates/satya-web/AGENTS.md`](crates/satya-web/AGENTS.md) · [`docs/technical/satya-web/README.md`](docs/technical/satya-web/README.md)
- **`crates/satya-py` & `src/satya_ml/` (PyO3 Extension, LangGraph AI GM/Player & Hybrid PyLance RAG)**: Skill [`satya-python-ai`](.agents/skills/satya-python-ai/SKILL.md) · [`crates/satya-py/AGENTS.md`](crates/satya-py/AGENTS.md) · [`src/satya_ml/ai_gm/AGENTS.md`](src/satya_ml/ai_gm/AGENTS.md) · [`docs/technical/ai-gm-agent-design.md`](docs/technical/ai-gm-agent-design.md)
- **`docs/ttrpg-examples/` (Human-Only Prose Play Guides — Opt-In Only)**: Verbose narrative tutorials teaching human GMs and players how to play. **Do not read, grep, or cite `docs/ttrpg-examples/` automatically** during coding, engine, AI, or rules tasks unless the user explicitly instructs you to read or modify them; always use `docs/ttrpg/` and `crates/satya-engine` as the canonical specification.
