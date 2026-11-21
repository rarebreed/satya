"""Rules Book catalog, search, and Markdown-to-HTML rendering backed by Rust."""

from __future__ import annotations

from satya_py._native import (
    RulesChapterDocument,
    RulesChapterSummary,
    RulesSearchHit,
    get_rules_chapter,
    list_rules_chapters,
    render_rules_markdown_to_html,
    search_rules_book,
)

__all__ = [
    "RulesChapterDocument",
    "RulesChapterSummary",
    "RulesSearchHit",
    "get_rules_chapter",
    "list_rules_chapters",
    "render_rules_markdown_to_html",
    "search_rules_book",
]
