# `docs/ttrpg-examples/` — Human-Only Prose Play Guides (Opt-In Only for AI Agents)

> [!IMPORTANT]
> **Opt-In Reading Rule for AI Agents & Research Subagents**:
> This directory contains verbose, conversational, human-oriented play examples written to teach new Game Masters and players how *Satya* works at a table.
> **DO NOT** automatically read, grep, summarize, or cite files in `docs/ttrpg-examples/` during coding, engine, AI GM, web, or rules-lookup tasks unless the user **explicitly instructs** you to create, read, review, or modify `docs/ttrpg-examples/`.
> For terse, canonical mechanics and exact formulas, always read [`docs/ttrpg/`](../ttrpg/SUMMARY.md) and [`crates/satya-engine/`](../../crates/satya-engine/src/lib.rs).

---

## Authoring Rules (When Explicitly Editing `docs/ttrpg-examples/`)

1. **Keep Chapters Modular & Readable (`~300–750` Lines / `<= 45 KB`)**:
   - Every Markdown file in this folder must stay within `<= 45,000` bytes (`<= 750` lines) and use strict `LF` (`\n`) line endings.
2. **Preserve 100% Mechanical Fidelity to `docs/ttrpg/` & `crates/satya-engine`**:
   - Never invent mechanics or alter canonical sign conventions (`Δ_int` positive = wider stumble penalty `6 + Δ_int`; `Δ_tool` and `Δ_ext` positive = harder target `TN_P = 31 - C_P + Δ_tool`, `TN_GM = 21 - Δ_ext`; direct Characteristic/Pool modifiers negative = penalty).
   - Every worked dice cascade, Tick cost, Tactical Triad allocation, ballistic value, and trauma step must reproduce the exact rules in [`docs/ttrpg/`](../ttrpg/SUMMARY.md) and [`crates/satya-engine/`](../../crates/satya-engine/src/lib.rs).
3. **Maintain RAG & Rules-Book Isolation**:
   - Do **not** register `docs/ttrpg-examples/` files in `STATIC_CHAPTERS` ([`crates/satya-engine/src/rules_book.rs`](../../crates/satya-engine/src/rules_book.rs)), `_CANONICAL_RULES_FILES` ([`src/satya_ml/ai_gm/tools.py`](../../src/satya_ml/ai_gm/tools.py)), `BOOK_CHAPTER_SPECS` ([`src/satya_ml/build_rules_book.py`](../../src/satya_ml/build_rules_book.py)), or `build_ttrpg_rules_chunks` ([`src/satya_ml/rag/corpus_builder.py`](../../src/satya_ml/rag/corpus_builder.py)) unless explicitly requested.
4. **Formatting Hygiene**:
   - Escape literal currency `$` in prose (`\$650`) or wrap dollar amounts in backticks (`` `$650` ``) so currency signs never collide with inline KaTeX/LaTeX math delimiters.
   - Always quote Mermaid node labels containing punctuation (`id["Label (Detail)"]`).
