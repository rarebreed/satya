---
name: satya-rust-workspace
description: >-
  Universal Rust architecture, modularization, and anti-bloat steering guide for all
  crates in the Satya workspace (satya-schema, satya-engine, satya-web, satya-py,
  satya-signaling, satya-storage). Activate this skill whenever creating, modifying,
  or refactoring any Rust (.rs), stylesheet (.css), or documentation (.md) file in
  the repository.
---

# Satya Rust Workspace Steering Skill (`satya-rust-workspace`)

This skill defines the universal architectural rules, file-size limits, modularity patterns, and documentation conventions that apply across **every** crate in the Satya Rust workspace (`crates/*`).

For crate-specific workflows and module maps, also activate:
- **`satya-rust-engine`**: [`.agents/skills/satya-rust-engine/SKILL.md`](../satya-rust-engine/SKILL.md) (and [`docs/technical/rust-engine-steering.md`](../../../docs/technical/rust-engine-steering.md))
- **`satya-rust-web`**: [`.agents/skills/satya-rust-web/SKILL.md`](../satya-rust-web/SKILL.md) (and [`docs/technical/rust-web-steering.md`](../../../docs/technical/rust-web-steering.md))

---

## 1. Hard File Size Ceiling (`<= 45 KB` / `< 46,080` Bytes)

1. **Every Source File Must Fit in a Single `read_file` Call**:
   - Every `.rs`, `.css`, and `.md` file in the repository must remain strictly **`<= 45 KB` (`<= 45,000` bytes)**—below the `46,080`-byte (`800`-line) single-call truncation limit of AI agent file-reading tools.
2. **Split Into Companion Modules Rather Than Deleting Features or Prose**:
   - When a file approaches `45 KB`, split it along clear domain, view, or pipeline boundaries into focused `15–42 KB` companion files rather than stripping functionality, tests, or documentation.
3. **Preserve Flat Re-Exports (`pub use companion::*;`) When Splitting**:
   - Whenever a module `foo.rs` is split into `foo.rs` + `foo_sub.rs`, re-export `pub use foo_sub::*;` in both `foo.rs` and the parent `mod.rs` / `lib.rs` so downstream crates and existing test suites compile with zero import-path churn.

---

## 2. Density Over Boilerplate

1. **Zero Parallel Domain Types or Duplicated Formulas**:
   - Never define a duplicate struct, enum, or mathematical formula in a downstream crate when a canonical definition already exists in `satya-schema` or `satya-engine`.
2. **Declarative Macros & Static Lookup Tables Over Repetitive Code**:
   - Replace repetitive registration blocks (such as `Closure::new` + `Reflect::set` + `forget`), repeated enum boilerplate (`as_str`, `label`, `from_code_str`), or repeated UI card markup with compact declarative macros (`bind_window_callbacks!`, `define_coded_enum!`) or `const` slice tables (`&'static [(...)]`).
   - Never heap-allocate a `Vec` at runtime just to iterate or look up a static catalog entry.
3. **Maximum 5–6 Positional Parameters**:
   - When a function requires more than 6 inputs (`clippy::too_many_arguments`), group related parameters into a typed configuration/context struct or derive labels from a shared base parameter.

---

## 3. High-Signal Documentation & Identifier Conventions

1. **High-Signal Docstrings (No `#![deny(missing_docs)]`)**:
   - Include a concise `//!` module header at the top of every `.rs` file explaining its responsibility and how it connects to sibling modules.
   - Include `///` docstrings on public entrypoints, non-obvious structs, and mathematical calculators (citing canonical formulas or `docs/ttrpg/` sections where applicable).
   - Do **not** enable `#![deny(missing_docs)]` or add filler docstrings on self-explanatory struct fields, enum variants, or internal plumbing.
2. **Encode Units & Scales Directly in Identifiers**:
   - Use explicit unit suffixes in field and variable names (`_ms`, `_us`, `_deg`, `_rad`, `_m`, `_cm`, `_mm`, `_kg`, `_grains`, `_joules`, `_liters`, `_sv`, `_usd`, `_ticks`, `_pct`) so units are unambiguous at every call site without needing docstrings.
3. **Use Optional `$(#[$meta:meta])*` in `macro_rules!`**:
   - Never force macro callers to supply artificial docstrings via `#[doc = $doc:literal]`; match optional attributes with `$(#[$meta:meta])*`.

---

## 4. Verification Workflow

After modifying any Rust crate:
1. **Verify File Sizes (`<= 45,000` bytes)**:
   ```powershell
   Get-ChildItem -Path crates -Recurse -File -Include *.rs,*.css,*.md | Where-Object { $_.Length -gt 45000 } | Select-Object FullName, Length
   ```
2. **Run Unit Tests & Clippy**:
   ```powershell
   cargo test --offline -p <crate-name>
   cargo clippy --offline -p <crate-name> --all-targets -- -D warnings
   ```
