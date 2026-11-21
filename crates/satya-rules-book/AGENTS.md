# `satya-rules-book` Agent Coding & Architecture Rules

`satya-rules-book` is a pure leaf crate (depends only on `satya-schema`, `serde`, and `serde_json`, plus optional `satya-engine` gated behind `--features book-builder` for `bin/build_rules_book.rs`).

1. **Bidirectional `docs/ttrpg/` <-> Rust Sync (5 Registries)**:
   - Whenever adding, renaming, or splitting any chapter in `docs/ttrpg/*.md` (embedded via `include_str!` in `src/lib.rs`), update:
     1. [`docs/ttrpg/SUMMARY.md`](../../docs/ttrpg/SUMMARY.md)
     2. `STATIC_CHAPTERS` in [`src/lib.rs`](src/lib.rs) and `resolve_markdown_link_chapter_id` in [`src/rules_book_html.rs`](src/rules_book_html.rs)
     3. Relative link map in [`src/bin/build_rules_book.rs`](src/bin/build_rules_book.rs)
     4. `BOOK_CHAPTER_SPECS` in [`src/satya_ml/build_rules_book.py`](../../src/satya_ml/build_rules_book.py) and `CHAPTER_SPECS` in [`notebooks/satya_rules_book.py`](../../notebooks/satya_rules_book.py)
     5. `_CANONICAL_RULES_FILES` in [`src/satya_ml/ai_gm/tools.py`](../../src/satya_ml/ai_gm/tools.py)
2. **Fast Scoped Verification**:
   - Verify `docs/ttrpg/*.md` or renderer edits with `pixi run test-rules-book` (~1s); verify the optional static-HTML builder CLI with `pixi run cargo clippy --offline -p satya-rules-book --features book-builder --all-targets -- -D warnings`.
   - Keep default features free of `satya-engine` so `docs/ttrpg/*.md` edits never trigger `satya-engine` / `satya-maps` rebuilds.
