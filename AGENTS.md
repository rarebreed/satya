# Satya Workspace Agent Coding & Architecture Rules

When working on any Rust crate or documentation file in this repository, activate the corresponding workspace skill in `.agents/skills/` and follow the universal rules in [`.agents/skills/satya-rust-workspace/SKILL.md`](.agents/skills/satya-rust-workspace/SKILL.md) ([`docs/technical/rust-workspace-steering.md`](docs/technical/rust-workspace-steering.md)):

- **All Rust Crates (`crates/*`)**: Skill [`satya-rust-workspace`](.agents/skills/satya-rust-workspace/SKILL.md) · [`docs/technical/rust-workspace-steering.md`](docs/technical/rust-workspace-steering.md)
  - Pragmatic file size target: aim to keep `.rs`, `.css`, and `.md` files reasonably sized (~`15–45 KB`) by splitting major features into focused companion modules with flat `pub use` re-exports upfront. Do **not** micromanage exact byte/line counts or re-edit files repeatedly just to shave off a few kilobytes.
  - Prefer declarative macros and `const` lookup tables over repetitive boilerplate.
  - Write high-signal `//!` module headers and `///` entrypoint docstrings (no `#![deny(missing_docs)]`), and encode physical/time units directly in field and parameter names (`_ms`, `_us`, `_deg`, `_m`, `_kg`).
- **`crates/satya-schema` & `schemas/v*/`**: [`crates/satya-schema/AGENTS.md`](crates/satya-schema/AGENTS.md)
- **`crates/satya-engine` & `docs/ttrpg/`**: Skill [`satya-rust-engine`](.agents/skills/satya-rust-engine/SKILL.md) · [`crates/satya-engine/AGENTS.md`](crates/satya-engine/AGENTS.md) · [`docs/technical/rust-engine-steering.md`](docs/technical/rust-engine-steering.md)
- **`crates/satya-web` (Leptos SSR + WebAssembly Client)**: Skill [`satya-rust-web`](.agents/skills/satya-rust-web/SKILL.md) · [`crates/satya-web/AGENTS.md`](crates/satya-web/AGENTS.md) · [`docs/technical/rust-web-steering.md`](docs/technical/rust-web-steering.md)
