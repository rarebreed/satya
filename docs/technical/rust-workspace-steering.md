# Rust Workspace Technical Steering Guide (`satya-rust-workspace`)

> [!NOTE]
> The canonical, agent-loaded rulebook for universal Satya Rust architecture, modularization (`~300–750` lines / `<= 45 KB`), `LF` (`\n`) endings, and offline Windows Cargo/PyO3/pytest commands lives in [`.agents/skills/satya-rust-workspace/SKILL.md`](../../.agents/skills/satya-rust-workspace/SKILL.md).

## Crate Rulebooks & Companion Guides

- **Root Index**: [`AGENTS.md`](../../AGENTS.md)
- **Schemas & Avro Wire Registry (`crates/satya-schema`)**: [`crates/satya-schema/AGENTS.md`](../../crates/satya-schema/AGENTS.md)
- **Game Engine & TTRPG Mechanics (`crates/satya-engine`)**: [`satya-rust-engine` Skill](../../.agents/skills/satya-rust-engine/SKILL.md) · [`crates/satya-engine/AGENTS.md`](../../crates/satya-engine/AGENTS.md)
- **Apache Iggy WAL & Avro Sinks (`crates/satya-storage`)**: [`crates/satya-storage/AGENTS.md`](../../crates/satya-storage/AGENTS.md)
- **Axum HTTP/WS Signaling & GM Vault (`crates/satya-signaling`)**: [`crates/satya-signaling/AGENTS.md`](../../crates/satya-signaling/AGENTS.md)
- **Leptos SSR + WebAssembly + WebGPU (`crates/satya-web`)**: [`satya-rust-web` Skill](../../.agents/skills/satya-rust-web/SKILL.md) · [`crates/satya-web/AGENTS.md`](../../crates/satya-web/AGENTS.md) · [`docs/technical/satya-web/README.md`](satya-web/README.md)
- **PyO3 Python Bindings (`crates/satya-py`)**: [`crates/satya-py/AGENTS.md`](../../crates/satya-py/AGENTS.md) · [`src/satya_ml/ai_gm/AGENTS.md`](../../src/satya_ml/ai_gm/AGENTS.md)
