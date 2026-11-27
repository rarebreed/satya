"""Tests for the static HTML Awakened Campaign Book generator and shared book themes."""

from __future__ import annotations

import json
from pathlib import Path
import re

from satya_ml.build_campaign_book import CAMPAIGN_CHAPTER_SPECS, build_campaign_book
from satya_ml.build_rules_book import build_rules_book

_REPO_ROOT = Path(__file__).resolve().parent.parent


def test_campaign_specs_cover_all_campaign_markdown_files() -> None:
    """Every Markdown file under `docs/campaign/` (except SUMMARY.md) is registered."""
    expected_paths = {
        p.relative_to(_REPO_ROOT / "docs" / "campaign").as_posix()
        for p in (_REPO_ROOT / "docs" / "campaign").rglob("*.md")
        if p.name.upper() != "SUMMARY.MD"
    }
    registered_paths = {spec["relative_path"] for spec in CAMPAIGN_CHAPTER_SPECS}
    assert registered_paths == expected_paths
    assert len(CAMPAIGN_CHAPTER_SPECS) == 21


def test_build_static_campaign_book_themes_links_and_assets(tmp_path: Path) -> None:
    """Verify 21-chapter Campaign Book build, Blue Lotus theme, Sangha Robes contrast, and links."""
    rules_out = tmp_path / "book"
    campaign_out = tmp_path / "campaign-book"

    rules_summary = build_rules_book(output_dir=rules_out, repo_root=_REPO_ROOT)
    campaign_summary = build_campaign_book(output_dir=campaign_out, repo_root=_REPO_ROOT)

    assert rules_summary["chapter_count"] == 24
    assert campaign_summary["chapter_count"] == 21
    assert len(campaign_summary["generated_files"]) == 23
    assert len(campaign_summary["asset_files"]) == 2

    css_text = (campaign_out / "satya_book.css").read_text(encoding="utf-8")
    js_text = (campaign_out / "satya_book.js").read_text(encoding="utf-8")

    # Verify all 4 themes exist and Sangha Robes is distinct from Chan Wood
    for theme_id in ("sangha-robes", "awakened-lotus", "chan-sanctuary", "tactical-hud"):
        assert theme_id in css_text
        assert f'"{theme_id}"' in js_text

    assert "--page-bg: #dfe3e6;" in css_text
    assert "--ink: #16191d;" in css_text
    assert "--page-bg: #f8f4ea;" in css_text
    assert "--page-bg: #f1f4f8;" in css_text
    assert 'window.location.pathname.includes("/campaign-book")' in js_text
    assert "getThemeKey()" in js_text

    index_html = (campaign_out / "index.html").read_text(encoding="utf-8")
    assert 'data-book-kind="awakened"' in index_html
    assert 'data-default-theme="awakened-lotus"' in index_html
    assert 'data-book-theme="awakened-lotus"' in index_html
    assert 'data-set-theme="awakened-lotus"' in index_html
    assert 'data-set-theme="sangha-robes"' in index_html
    assert 'data-set-theme="chan-sanctuary"' in index_html
    assert 'data-set-theme="tactical-hud"' in index_html
    assert 'href="../book/index.html"' in index_html
    assert "Blue Lotus" in index_html

    rules_index_html = (rules_out / "index.html").read_text(encoding="utf-8")
    assert 'href="../campaign-book/index.html"' in rules_index_html

    catalog = json.loads((campaign_out / "catalog.json").read_text(encoding="utf-8"))
    assert catalog["chapter_count"] == 21
    assert len(catalog["chapters"]) == 21

    # Verify chapter HTML rendering, Mermaid, LaTeX, images, and cross-book links
    affective_html = (campaign_out / "affective_telemetry_and_vector_clamping.html").read_text(
        encoding="utf-8"
    )
    assert "rules-math-block" in affective_html
    assert "&amp;amp;" not in affective_html

    chronicle_html = (campaign_out / "three_arc_chronicle.html").read_text(encoding="utf-8")
    assert "rules-mermaid" in chronicle_html
    assert "mermaid-node" in chronicle_html
    assert 'data-break="page"' in chronicle_html

    # Verify TTRPG-style layout directives, callout boxes, and pagination breaks
    assert "rules-figure--full" in (campaign_out / "prologue.html").read_text(encoding="utf-8")
    ep1_html = (campaign_out / "episode_01_st_jude_annex.html").read_text(encoding="utf-8")
    assert "rules-figure--full" in ep1_html
    assert "rules-dossier" in ep1_html
    assert "rules-example" in ep1_html
    assert "rules-sidebar" in ep1_html

    assert "rules-figure--spread" in (campaign_out / "geopolitics_and_nations.html").read_text(
        encoding="utf-8"
    )
    assert "rules-figure--spread" in (
        campaign_out / "political_economy_token_tax_and_unrest.html"
    ).read_text(encoding="utf-8")

    factions_html = (campaign_out / "factions_of_2035.html").read_text(encoding="utf-8")
    assert "rules-figure--column" in factions_html
    assert "rules-statblock" in factions_html
    assert "rules-figure--column" in (
        campaign_out / "ai_2_0_substrate_and_active_inference.html"
    ).read_text(encoding="utf-8")
    assert "rules-statblock" in (campaign_out / "satyang_and_four_pillars.html").read_text(
        encoding="utf-8"
    )
    assert "rules-figure--half-top" in (
        campaign_out / "polycrisis_economy_and_society.html"
    ).read_text(encoding="utf-8")
    polycrisis_html = (campaign_out / "polycrisis_economy_and_society.html").read_text(
        encoding="utf-8"
    )
    assert '<svg class="mermaid-svg"' in polycrisis_html
    assert 'class="mermaid-node"' in polycrisis_html
    assert 'class="mermaid-edge' in polycrisis_html
    assert "Social Media Tribal Bubbles" in polycrisis_html
    assert r"\n" not in polycrisis_html
    assert ".mermaid-svg" in css_text

    history_html = (campaign_out / "history_and_ai_revolution.html").read_text(encoding="utf-8")
    assert "rules-figure--quarter" in history_html
    assert "rules-sidebar" in history_html
    assert "rules-sidebar" in (campaign_out / "ai_minds_and_consciousness.html").read_text(
        encoding="utf-8"
    )
    assert 'data-break="page"' in (
        campaign_out / "authors_note_to_researchers_and_future_minds.html"
    ).read_text(encoding="utf-8")

    existing_campaign_html = {p.name for p in campaign_out.glob("*.html")}
    existing_rules_html = {p.name for p in rules_out.glob("*.html")}

    for html_file in campaign_out.glob("*.html"):
        text = html_file.read_text(encoding="utf-8")
        assert "&amp;amp;" not in text
        for href in re.findall(r'href="([^"]+)"', text):
            if href.startswith(("http://", "https://", "#", "mailto:")):
                continue
            target = href.split("#", 1)[0]
            if not target:
                continue
            if target.startswith("../book/"):
                assert target[len("../book/"):] in existing_rules_html, f"{html_file.name}: missing {href}"
            elif target.endswith(".html"):
                assert target in existing_campaign_html, f"{html_file.name}: missing {href}"
        for src in re.findall(r'src="([^"]+)"', text):
            if src.startswith("../assets/"):
                asset_path = _REPO_ROOT / "docs" / "assets" / src[len("../assets/"):]
                assert asset_path.is_file(), f"{html_file.name}: missing image {src}"
