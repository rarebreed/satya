# Satya Workspace Agent Coding & Architecture Rules

When working on any Rust crate or documentation file in this repository, activate the corresponding workspace skill in `.agents/skills/` and follow the universal rules in [`.agents/skills/satya-rust-workspace/SKILL.md`](.agents/skills/satya-rust-workspace/SKILL.md) ([`docs/technical/rust-workspace-steering.md`](docs/technical/rust-workspace-steering.md)):

- **All Rust Crates (`crates/*`)**: Skill [`satya-rust-workspace`](.agents/skills/satya-rust-workspace/SKILL.md) · [`docs/technical/rust-workspace-steering.md`](docs/technical/rust-workspace-steering.md)
  - Hard file size ceiling: every `.rs`, `.css`, and `.md` file must remain **`<= 45 KB` (`< 46,080` bytes)** so it can be read in a single `read_file` call without truncation.
  - Split growing files into focused companion modules with flat `pub use` re-exports rather than deleting functionality or documentation.
  - Prefer declarative macros and `const` lookup tables over repetitive boilerplate.
  - Write high-signal `//!` module headers and `///` entrypoint docstrings (no `#![deny(missing_docs)]`), and encode physical/time units directly in field and parameter names (`_ms`, `_us`, `_deg`, `_m`, `_kg`).
- **`crates/satya-engine` & `docs/ttrpg/`**: Skill [`satya-rust-engine`](.agents/skills/satya-rust-engine/SKILL.md) · [`crates/satya-engine/AGENTS.md`](crates/satya-engine/AGENTS.md) · [`docs/technical/rust-engine-steering.md`](docs/technical/rust-engine-steering.md)
- **`crates/satya-web` (Leptos SSR + WebAssembly Client)**: Skill [`satya-rust-web`](.agents/skills/satya-rust-web/SKILL.md) · [`crates/satya-web/AGENTS.md`](crates/satya-web/AGENTS.md) · [`docs/technical/rust-web-steering.md`](docs/technical/rust-web-steering.md)
