---
name: satya-rust-workspace
description: >-
  Universal Rust architecture, modularization, and anti-bloat steering guide for all
  crates in the Satya workspace (satya-schema, satya-engine, satya-web, satya-py,
  satya-signaling, satya-storage).
---

# Rust Workspace Technical Steering Guide (All Crates)

This document defines the universal architectural standards, file-size limits, modularity rules, and documentation conventions that apply across **every** crate in the Satya Rust workspace (`satya-schema`, `satya-engine`, `satya-web`, `satya-py`, `satya-signaling`).

Crate-specific steering guides build on top of this document:
- **Schemas, Wire Framing & GDPR (`crates/satya-schema`)**: [`crates/satya-schema/AGENTS.md`](../../crates/satya-schema/AGENTS.md)
- **Game Engine & TTRPG Domain (`crates/satya-engine`)**: [`docs/technical/rust-engine-steering.md`](rust-engine-steering.md) & [`crates/satya-engine/AGENTS.md`](../../crates/satya-engine/AGENTS.md)
- **Apache Iggy WAL & Avro Batch Storage (`crates/satya-storage`)**: [`crates/satya-storage/AGENTS.md`](../../crates/satya-storage/AGENTS.md)
- **Axum Signaling, Auth, GM Vault & AI GM Bridge (`crates/satya-signaling`)**: [`crates/satya-signaling/AGENTS.md`](../../crates/satya-signaling/AGENTS.md)
- **Full-Stack Leptos SSR + WebAssembly Client (`crates/satya-web`)**: [`docs/technical/rust-web-steering.md`](rust-web-steering.md) & [`crates/satya-web/AGENTS.md`](../../crates/satya-web/AGENTS.md)
- **PyO3 Python Extension (`crates/satya-py`)**: [`crates/satya-py/AGENTS.md`](../../crates/satya-py/AGENTS.md)

---

## 1. Hard File Size Ceiling (`<= 45 KB` / `< 46,080` Bytes)

1. **Every Source File Must Fit in a Single Agent Read**:
   - Every `.rs`, `.css`, and `.md` file in the repository must remain strictly **`<= 45 KB` (`<= 45,000` bytes)**—well below the `46,080`-byte (`800`-line) single-call truncation limit of AI agent file-reading tools.
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
   - When a function requires more than 6 inputs, group related parameters into a typed configuration/context struct.

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

## 4. Test Organization & Windows Editing Safety

1. **Dedicated Test Modules (`src/tests*.rs`)**:
   - Keep large unit and integration test suites in dedicated `src/tests.rs` (and `src/tests_*.rs`) modules rather than appending 500+ lines of `#[cfg(test)]` to the bottom of production source files.
2. **Strict `LF` (`\n`) Line-Ending & Windows Escaping Discipline**:
   - **Always write and save all repository files (`.rs`, `.py`, `.md`, `.css`, `.toml`, `.avsc`, `.json`, `.html`) with `LF` (`\n`) line endings — never `CRLF` (`\r\n`)**.
   - Mixing `CRLF` and `LF` breaks exact string matching in `edit_file`, bloats diffs, and causes cross-platform hash/fingerprint mismatches. If an existing file on Windows contains `CRLF` (`\r\n`) line endings, normalize it to `LF` (`\n`) before or during editing (e.g. `path.write_bytes(path.read_bytes().replace(b"\r\n", b"\n"))` or `newline="\n"` in Python).
   - On Windows, never use PowerShell `Set-Content` / `Out-File` (which default to `CRLF`) or double-quoted strings (`"..."`) to write Rust or Markdown containing backticks or `$` variables, as PowerShell strips backtick escapes and interpolates `$var`. Prefer structured file-editing tools (`write_file` / `edit_file`) or UTF-8 scripts explicitly configured with `LF` (`\n`) line endings.
