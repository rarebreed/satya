---
name: satya-rust-workspace
description: >-
  Universal Rust architecture, modularization, and anti-bloat steering guide for all
  crates in the Satya workspace (satya-schema, satya-engine, satya-web, satya-py,
  satya-signaling, satya-storage). Activate this skill whenever creating, modifying,
  or refactoring any Rust (.rs), Python (.py), stylesheet (.css), or documentation
  (.md) file in the repository.
---

# Satya Rust Workspace Steering Skill (`satya-rust-workspace`)

Canonical universal rules for **every** crate in the Satya Rust workspace (`crates/*`).

Domain-specific workflows and module maps:
- **`satya-rust-engine`**: [`.agents/skills/satya-rust-engine/SKILL.md`](../satya-rust-engine/SKILL.md)
- **`satya-rust-web`**: [`.agents/skills/satya-rust-web/SKILL.md`](../satya-rust-web/SKILL.md)
- **`satya-python-ai`**: [`.agents/skills/satya-python-ai/SKILL.md`](../satya-python-ai/SKILL.md)

---

## 1. Single-Read Modularization (`~300–750` Lines / `<= 45 KB`)

1. **Design Focused Submodules Upfront**:
   - Aim to keep `.rs`, `.py`, `.css`, and `.md` files within **`~300–750` lines (`<= 45 KB`)** so every file reads cleanly inside a single `800`-line / `46,080`-byte `read_file` call.
   - When a file outgrows `~750` lines, split along domain/view boundaries into companion modules rather than golf-compressing code or re-editing over a few kilobytes.
2. **Flat Companion Re-Exports Without Duplicate Glob Imports**:
   - When splitting `foo.rs` into `foo.rs` + `foo_sub.rs`, put `pub use crate::...::foo_sub::*;` in `foo.rs` and declare `pub mod foo_sub;` in the parent `lib.rs` / `mod.rs` (keeping `pub use foo::*;` in `lib.rs` / `mod.rs` without ALSO adding `pub use foo_sub::*;` there, which triggers `unused_imports` under `clippy -- -D warnings`).

---

## 2. Density Over Boilerplate

1. **Zero Parallel Domain Types or Duplicated Formulas**:
   - Never redefine a struct, enum, geometry helper, or mathematical formula in a downstream crate (`satya-web`, `satya-signaling`, `satya-py`, `src/satya_ml/`) when a canonical definition already exists in `satya-schema` or `satya-engine` (`MapPropPlacement::footprint_m()`, `PropFootprintM`).
2. **Declarative Macros & Static Lookup Tables Over Repetitive Code**:
   - Use `satya_schema::define_wire_enum!` for `SCREAMING_SNAKE_CASE` wire enums (`aliases`, `as_str`, `code`, `label`, `from_code_str`), `satya_engine::define_coded_enum!` for domain enums, `bind_window_callbacks!` for Wasm globals, and `&'static [...]` `const` slice tables instead of runtime heap `Vec` catalogs.
3. **Maximum 5–6 Positional Parameters**:
   - When a function requires more than 6 inputs (`clippy::too_many_arguments`), group related parameters into a typed configuration/context struct.

---

## 3. High-Signal Documentation & Identifier Conventions

1. **High-Signal Docstrings (No `#![deny(missing_docs)]`)**:
   - Include a concise `//!` module header on every `.rs` file and `///` docstrings on public entrypoints, non-obvious structs, and mathematical calculators.
   - Never enable `#![deny(missing_docs)]` or add filler docstrings on self-explanatory fields or internal plumbing.
2. **Encode Units & Scales Directly in Identifiers**:
   - Use explicit unit suffixes (`_ms`, `_us`, `_deg`, `_rad`, `_m`, `_cm`, `_mm`, `_kg`, `_grains`, `_joules`, `_liters`, `_sv`, `_usd`, `_ticks`, `_pct`).
3. **Use Optional `$(#[$meta:meta])*` in `macro_rules!`**:
   - Never require artificial docstrings via `#[doc = $doc:literal]`.

---

## 4. Editing Hygiene, Strict `LF` (`\n`), & Cross-Crate Compilation

1. **Always Use `LF` (`\n`) — Never `CRLF` (`\r\n`)**:
   - Every created or modified file (`.rs`, `.py`, `.md`, `.css`, `.toml`, `.avsc`, `.json`, `.html`) **must** use `LF` (`\n`). Normalize any `CRLF` file on Windows via `path.write_bytes(path.read_bytes().replace(b"\r\n", b"\n"))`; never use PowerShell `Set-Content` / `Out-File` or `"..."` strings containing backticks/`$`.
2. **Never Issue Parallel `edit_file` Calls on the Same File**:
   - On Windows, apply multiple edits to different regions of the same file sequentially, or combine them safely so stale targets/handles cannot conflict.
3. **Run Cross-Crate `cargo check --offline` Early**:
   - After modifying any shared DTO (`satya-schema`) or engine signature (`satya-engine`), immediately run `cargo check --offline` across downstream crates before writing more code.

---

## 5. Pixi Verification & Git Worktree Workflow

Always invoke tools via `pixi run ...` (never call `.pixi/envs/default/*.exe` directly or hardcode external `uv` Python paths). Run Cargo invocations sequentially so `target/` locks do not contend:

- **Core Rust Workspace (5 non-PyO3 crates)**:
  ```powershell
  pixi run cargo clippy --offline -p satya-schema -p satya-engine -p satya-web -p satya-signaling -p satya-storage --all-targets -- -D warnings
  pixi run cargo test --offline -p satya-schema -p satya-engine -p satya-web -p satya-signaling -p satya-storage
  ```
- **PyO3 Extension (`satya-py`)**: use `--no-default-features` (`auto-initialize` off) for Rust unit tests/clippy, and `pixi run build-py` to compile release `src/satya_py.pyd`:
  ```powershell
  pixi run cargo clippy --offline --no-default-features -p satya-py --all-targets -- -D warnings
  pixi run cargo test --offline --no-default-features -p satya-py
  pixi run build-py
  ```
- **Python Static Check, RAG Index & Parallel Pytest (`pytest-xdist`)**:
  ```powershell
  pixi run pyrefly check src/satya_ml tests
  pixi run build-rag
  pixi run test-py
  ```
- **Git Worktrees**: Branch worktrees (`worktrees/satya/<branch>`) reference the primary `.git` directory where `main` is checked out (`git worktree list`). Never run `git checkout main` inside a branch worktree; merge `main` into a feature worktree with `git merge main`, and merge a feature branch back into `main` via `git -C <main-worktree-path> merge <branch>`.
