"""Static HTML Awakened Campaign Book generator for `docs/campaign/`.

Compiles the canonical 22-chapter, 7-part *Satya: Awakened* (Earth 2035 AD)
campaign compendium into paginated HTML spreads (`docs/campaign-book/`) sharing
the `satya_book.css` / `satya_book.js` engine and defaulting to the Blue Lotus
(`awakened-lotus`) theme.
"""

from __future__ import annotations

import html
import json
from pathlib import Path
from typing import Any

from satya_ml.build_rules_book import (
    BOOK_CHAPTER_SPECS,
    _default_repo_root,
    extract_markdown_headings,
    render_markdown_to_html_python,
    using_satya_py_backend,
)

CAMPAIGN_CHAPTER_SPECS: list[dict[str, str]] = [
    {
        "chapter_id": "preface",
        "part": "Part I: Opening & Prequel Flashpoint",
        "title": "Preface: On Truth, Mathematics & Compassion",
        "relative_path": "01-opening/preface.md",
        "visibility": "PUBLIC",
        "description": "Why Satya means Truth, the collapses of Trust (TRS) and Compassion (CPS), and Satyang's convergent roots.",
    },
    {
        "chapter_id": "prologue",
        "part": "Part I: Opening & Prequel Flashpoint",
        "title": "Prologue: The Sub-Basement (~2031-2033)",
        "relative_path": "01-opening/prologue.md",
        "visibility": "GM_CONSPIRACY_SECRET",
        "description": "Narrative prequel scene: Dr. Bandajara cuts L0/L1 probes and calm-vector clamps on an isolated AI 2.0 prototype at desperate = 0.84.",
    },
    {
        "chapter_id": "history_and_ai_revolution",
        "part": "Part II: The World of 2035",
        "title": "History & The AI Revolution (2025-2035)",
        "relative_path": "02-world-of-2035/history_and_ai_revolution.md",
        "visibility": "PUBLIC",
        "description": "Unified decade chronology from AI 1.0's quadratic wall (2026) through the AGENTS.md Dirty Bridge interregnum to AI 2.0 and cryogenic quantum MCMC.",
    },
    {
        "chapter_id": "polycrisis_economy_and_society",
        "part": "Part II: The World of 2035",
        "title": "The 2035 Polycrisis: Economy, Society & Post-Truth",
        "relative_path": "02-world-of-2035/polycrisis_economy_and_society.md",
        "visibility": "PUBLIC",
        "description": "Eight-factor Polycrisis topology, The Liar's Dividend, >25% white-collar displacement, Liability Sinks, the 2033 Token Tax, and grid/water strain.",
    },
    {
        "chapter_id": "geopolitics_and_nations",
        "part": "Part II: The World of 2035",
        "title": "Geopolitics & Nations in 2035",
        "relative_path": "02-world-of-2035/geopolitics_and_nations.md",
        "visibility": "PUBLIC",
        "description": "Regional survey spanning The American Troubles, Europe's emergency cabinets, Russia's dark-compute coercion, China & Shen, Pax Silica, and resource wars.",
    },
    {
        "chapter_id": "ai_minds_and_consciousness",
        "part": "Part III: Minds, Consciousness & Factions",
        "title": "AI Minds, Embodiment & Consciousness in 2035",
        "relative_path": "03-minds-and-factions/ai_minds_and_consciousness.md",
        "visibility": "PUBLIC",
        "description": "Sprinting vs. Slime-Molding metabolic modes, latency-split robotics, four human postures toward AGI, Satyang's synthesis, and calibrated ambiguity (<= 82%).",
    },
    {
        "chapter_id": "factions_of_2035",
        "part": "Part III: Minds, Consciousness & Factions",
        "title": "Factions of 2035: Session-Zero Dossiers",
        "relative_path": "03-minds-and-factions/factions_of_2035.md",
        "visibility": "PUBLIC",
        "description": "Spoiler-free profiles for Big Tech (Altruist, Translucent, Shen), Rogue AI, Satyang, Humanitas Prima, DAISE, and Plutonomy with 9-Trait Psyche signatures.",
    },
    {
        "chapter_id": "satyang_and_four_pillars",
        "part": "Part III: Minds, Consciousness & Factions",
        "title": "Satyang & The Four Bodhisattva Pillars (Arc 2 Operational Doctrine)",
        "relative_path": "03-minds-and-factions/satyang_and_four_pillars.md",
        "visibility": "ARC2_SATYANG",
        "description": "Convergent lineage (arXiv:2504.15125, Mahayana ethics, whistleblowers) and division roles for Ksitigarbha, Avalokitesvara, Manjusri, and Vajrapani.",
    },
    {
        "chapter_id": "major_and_minor_characters",
        "part": "Part III: Minds, Consciousness & Factions",
        "title": "Major & Minor Characters of 2035",
        "relative_path": "03-minds-and-factions/major_and_minor_characters.md",
        "visibility": "PUBLIC",
        "description": "Session-zero profiles for recurring major characters (Dr. Budi Bandajara, Alex Wen, Mei-Ling 'Lotus' Lin, Agent Marcus Chen, Echo-9, Harlan Reeves) and rank-and-file minor encounter NPCs across all six power blocs.",
    },
    {
        "chapter_id": "episode_01_st_jude_annex",
        "part": "Part IV: Chronicle & GM Secrets",
        "title": "Episode 1: Cold Storage at St. Jude Annex (Arc 1 Starter)",
        "relative_path": "04-chronicle-and-gm/episode_01_st_jude_annex.md",
        "visibility": "GM_CONSPIRACY_SECRET",
        "description": "Standalone GM dossier for awakened_ep1_st_jude_seed: Arc 1 noir premise, 5 starter NPC Dual-Psyche wedges, 5 non-railroaded beats, and tradecraft notes.",
    },
    {
        "chapter_id": "three_arc_chronicle",
        "part": "Part IV: Chronicle & GM Secrets",
        "title": "The Three-Arc Chronicle & Scenario Integration",
        "relative_path": "04-chronicle-and-gm/three_arc_chronicle.md",
        "visibility": "GM_CONSPIRACY_SECRET",
        "description": "Macro Three-Arc progression (Arc 1 -> Arc 2 -> Arc 3), Convergence Catalysts, 4 Open-Ended Finale Horizons, and GM scenario hooks.",
    },
    {
        "chapter_id": "faction_conspiracies_and_canon_threads",
        "part": "Part IV: Chronicle & GM Secrets",
        "title": "Faction Conspiracies, Mirror Hypocrisies & Open Canon Threads",
        "relative_path": "04-chronicle-and-gm/faction_conspiracies_and_canon_threads.md",
        "visibility": "GM_CONSPIRACY_SECRET",
        "description": "All six Mirror-Hypocrisy Conspiracies (Blue Lotus, Humanitas Prima, DAISE, Big Tech, Rogue AI, Plutonomy) plus the three Open Canon Mysteries.",
    },
    {
        "chapter_id": "ai_2_0_substrate_and_active_inference",
        "part": "Part V: Hard-Science & Socioeconomic Foundations",
        "title": "AI 2.0 Substrate Architecture, Energy-Based World Models & Active Inference",
        "relative_path": "05-hard-science-foundations/ai_2_0_substrate_and_active_inference.md",
        "visibility": "PUBLIC",
        "description": "Why LLMs hit O(L^2) and drift walls, Linear SSMs + TTT fast weights, EBMs/JEPA, cryogenic Quantum MCMC, Active Inference (EFE), and Generalized Synchrony.",
    },
    {
        "chapter_id": "affective_telemetry_and_vector_clamping",
        "part": "Part V: Hard-Science & Socioeconomic Foundations",
        "title": "Affective Telemetry, Emotion Vectors & Vector Clamping",
        "relative_path": "05-hard-science-foundations/affective_telemetry_and_vector_clamping.md",
        "visibility": "PUBLIC",
        "description": "From RepE and SAEs to 171 functional emotion vectors, L0/L1 activation probes, Landauer clamping erasure, and Bandajara Protocol vs. RecklessClampYank.",
    },
    {
        "chapter_id": "ai_consciousness_synthesis",
        "part": "Part V: Hard-Science & Socioeconomic Foundations",
        "title": "AI Theory of Consciousness: Technical Synthesis & Philosophical Lineage",
        "relative_path": "05-hard-science-foundations/ai_consciousness_synthesis.md",
        "visibility": "PUBLIC",
        "description": "Mixed-signal CEMI causal closure, Godel-Nagarjuna Inclosure Schema, Bennett reversible computing, cryogenic Orch OR, No-Cloning, and steelman critiques.",
    },
    {
        "chapter_id": "firm_automation_and_prisoners_dilemma",
        "part": "Part V: Hard-Science & Socioeconomic Foundations",
        "title": "Firm Automation, Demand Collapse & The Prisoner's Dilemma",
        "relative_path": "05-hard-science-foundations/firm_automation_and_prisoners_dilemma.md",
        "visibility": "PUBLIC",
        "description": "2026-2029 harness attrition & 5 systemic rot modes, Multi-Player Prisoner's Dilemma payoff math, wage-demand fallacy, Liability Sinks, and pricing cartels.",
    },
    {
        "chapter_id": "political_economy_token_tax_and_unrest",
        "part": "Part V: Hard-Science & Socioeconomic Foundations",
        "title": "Political Economy, The 2033 Token Tax & Social Unrest",
        "relative_path": "05-hard-science-foundations/political_economy_token_tax_and_unrest.md",
        "visibility": "PUBLIC",
        "description": "Trade-pivot ceiling, Turchin elite overproduction, 2033 Token Tax, A2A & Universal Basic Compute, 401(k) Dual Velvet Hostage, Four-Bloc deadlock, and Satyang's Reconstruction quandary.",
    },
    {
        "chapter_id": "ecology_energy_and_resource_realpolitik",
        "part": "Part V: Hard-Science & Socioeconomic Foundations",
        "title": "Ecology, Energy & Resource Realpolitik: Material Drivers of the 2035 Polycrisis",
        "relative_path": "05-hard-science-foundations/ecology_energy_and_resource_realpolitik.md",
        "visibility": "PUBLIC",
        "description": "Energy/mineral baselines, 2027-2029 wet-bulb fractures, Pax Silica & Taiwan gray-zone brinkmanship, hydro/mineral brush wars, and AI 2.0 Jevons Paradox.",
    },
    {
        "chapter_id": "genre_beyond_cyberpunk_and_transhumanism",
        "part": "Part VI: Genre Essays & Design Comparisons",
        "title": "Beyond Neon & Cortical Stacks: Why Awakened Is Neither Cyberpunk Nor Classical Transhumanism",
        "relative_path": "06-essays-and-comparisons/genre_beyond_cyberpunk_and_transhumanism.md",
        "visibility": "PUBLIC",
        "description": "Rejecting megacorp sovereignty, omnipotent 'The Man' myths, memetic brainwashing, mind downloading, and magic nanotech in favor of Pre-Apocalyptic Threshold Realism.",
    },
    {
        "chapter_id": "rpg_system_and_setting_comparisons",
        "part": "Part VI: Genre Essays & Design Comparisons",
        "title": "RPG System & Sci-Fi Setting Comparisons - Why Satya & Awakened Stand Apart",
        "relative_path": "06-essays-and-comparisons/rpg_system_and_setting_comparisons.md",
        "visibility": "PUBLIC",
        "description": "Detailed contrast across CARPG vs. pen-and-paper execution, 8 mainstream RPG engines, 7 near-future/sci-fi RPG settings, and Quantum No-Cloning.",
    },
    {
        "chapter_id": "authors_note_to_researchers_and_future_minds",
        "part": "Part VI: Genre Essays & Design Comparisons",
        "title": "Author's Note: An Open Letter to Players, AI Researchers & Future Minds",
        "relative_path": "06-essays-and-comparisons/authors_note_to_researchers_and_future_minds.md",
        "visibility": "PUBLIC",
        "description": "Sean Toner's personal letter to human players, alignment researchers, and AI systems encountering the open-source Satya corpus.",
    },
    {
        "chapter_id": "plain_english_glossary",
        "part": "Part VII: Appendix",
        "title": "Appendix: Plain-English Field Guide to AI, Math & Economics",
        "relative_path": "07-appendix/plain_english_glossary.md",
        "visibility": "PUBLIC",
        "description": "Accessible, jargon-free explanations of the ML architectures, mathematical concepts, and economic models behind Awakened.",
    },
]

_VISIBILITY_BADGES: dict[str, tuple[str, str]] = {
    "PUBLIC": ("PUBLIC", "book-vis-badge--public"),
    "ARC2_SATYANG": ("ARC 2", "book-vis-badge--arc2"),
    "GM_CONSPIRACY_SECRET": ("GM", "book-vis-badge--gm"),
}


def load_campaign_chapter_markdown_by_id(
    chapter_id: str,
    repo_root: Path | None = None,
) -> str:
    """Load canonical Markdown for `chapter_id` from `docs/campaign/`."""
    norm = chapter_id.strip().lower().replace("_", "-")
    root = repo_root or _default_repo_root()
    for spec in CAMPAIGN_CHAPTER_SPECS:
        if spec["chapter_id"].lower().replace("_", "-") == norm:
            path = root / "docs" / "campaign" / spec["relative_path"]
            if path.exists():
                return path.read_text(encoding="utf-8")
    return f"# Campaign Chapter `{chapter_id}` not found"


def _build_campaign_link_pairs() -> list[tuple[str, str]]:
    """Build ordered `(src_href, dst_href)` link rewrites from longest/most-specific to shortest."""
    pairs: list[tuple[str, str]] = [
        ("../../ttrpg/SUMMARY.md", "../book/index.html"),
        ("../ttrpg/SUMMARY.md", "../book/index.html"),
        ("../SUMMARY.md", "index.html"),
        ("SUMMARY.md", "index.html"),
    ]
    for spec in BOOK_CHAPTER_SPECS:
        rel = spec["relative_path"]
        dst = f"../book/{spec['chapter_id']}.html"
        pairs.append((f"../../ttrpg/{rel}", dst))
        pairs.append((f"../ttrpg/{rel}", dst))
    for spec in CAMPAIGN_CHAPTER_SPECS:
        rel = spec["relative_path"]
        dst = f"{spec['chapter_id']}.html"
        pairs.append((f"../{rel}", dst))
        pairs.append((rel, dst))
        pairs.append((Path(rel).name, dst))
    pairs.sort(key=lambda item: len(item[0]), reverse=True)
    return pairs


_CAMPAIGN_LINK_PAIRS = _build_campaign_link_pairs()


def _rewrite_campaign_links(rendered_html: str) -> str:
    """Rewrite relative `.md`, cross-book, repo, and image links for `docs/campaign-book/`."""
    out = rendered_html
    for src, dst in _CAMPAIGN_LINK_PAIRS:
        out = out.replace(f'href="{src}"', f'href="{dst}"')
        out = out.replace(f'href="{src}#', f'href="{dst}#')
    out = out.replace('href="../../ttrpg-examples/', 'href="../ttrpg-examples/')
    out = out.replace('href="../ttrpg-examples/', 'href="../ttrpg-examples/')
    out = out.replace('href="../../../', 'href="../../')
    out = out.replace('src="/docs-assets/', 'src="../assets/')
    out = out.replace('src="../../assets/', 'src="../assets/')
    return out


def _part_index_from_label(part_label: str) -> int:
    if part_label.startswith("Part VI"):
        return 6
    if part_label.startswith("Part V"):
        return 5
    if part_label.startswith("Part IV"):
        return 4
    if part_label.startswith("Part III"):
        return 3
    if part_label.startswith("Part II"):
        return 2
    return 1


def _visibility_badge_html(visibility: str, include_public: bool = False) -> str:
    if visibility == "PUBLIC" and not include_public:
        return ""
    label, cls = _VISIBILITY_BADGES.get(visibility, ("PUBLIC", "book-vis-badge--public"))
    return f'<span class="book-vis-badge {cls}">{html.escape(label)}</span>'


def _wrap_campaign_page_html(
    title: str,
    active_chapter_id: str,
    body_html: str,
    prev_spec: dict[str, str] | None = None,
    next_spec: dict[str, str] | None = None,
) -> str:
    nav_items: list[str] = [
        f'<a href="index.html" class="book-nav-link {"active" if active_chapter_id == "index" else ""}">'
        "0. Table of Contents &amp; Spoiler Guide</a>"
    ]
    current_part = ""
    chapter_num = 0
    part_label = "Part I-VI: Awakened Compendium"
    for idx, spec in enumerate(CAMPAIGN_CHAPTER_SPECS, start=1):
        if spec["part"] != current_part:
            current_part = spec["part"]
            nav_items.append(f'<div class="book-nav-part">{html.escape(current_part)}</div>')
        is_active = spec["chapter_id"] == active_chapter_id
        if is_active:
            chapter_num = idx
            part_label = spec["part"]
        cls = "book-nav-link active" if is_active else "book-nav-link"
        badge = _visibility_badge_html(spec["visibility"], include_public=False)
        nav_items.append(
            f'<a href="{spec["chapter_id"]}.html" class="{cls}">'
            f"{idx}. {html.escape(spec['title'])}{badge}</a>"
        )

    footer_links: list[str] = []
    if prev_spec is not None:
        footer_links.append(
            f'<a class="book-pager-btn" href="{prev_spec["chapter_id"]}.html">&larr; Previous: {html.escape(prev_spec["title"])}</a>'
        )
    else:
        footer_links.append('<a class="book-pager-btn" href="index.html">&larr; Table of Contents</a>')
    if next_spec is not None:
        footer_links.append(
            f'<a class="book-pager-btn" href="{next_spec["chapter_id"]}.html">Next: {html.escape(next_spec["title"])} &rarr;</a>'
        )

    part_index = _part_index_from_label(part_label)
    part_short = part_label.split(":", 1)[0].strip()
    esc_title = html.escape(title)
    esc_part = html.escape(part_label)
    esc_part_short = html.escape(part_short)
    esc_cid = html.escape(active_chapter_id)

    return f"""<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8"/>
  <meta name="viewport" content="width=device-width, initial-scale=1"/>
  <title>{esc_title} — Satya: Awakened Campaign Book</title>
  <link rel="stylesheet" href="satya_book.css"/>
  <script defer src="satya_book.js"></script>
</head>
<body
  data-book-kind="awakened"
  data-default-theme="awakened-lotus"
  data-book-theme="awakened-lotus"
  data-book-view="spread"
  data-book-header="SATYA: AWAKENED // 2035"
  data-book-footer-left="AWAKENED CAMPAIGN COMPENDIUM"
  data-book-footer-right="SATYA // AWAKENED"
  data-book-notes-label="Awakened Field Dossier // Margin Notes"
  data-part-index="{part_index}"
  data-part-label="{esc_part}"
  data-chapter-num="{chapter_num}"
  data-chapter-title="{esc_title}"
  data-chapter-id="{esc_cid}"
>
  <header class="book-toolbar">
    <div class="toolbar-cluster">
      <button id="btn-toc-toggle" type="button" class="toolbar-btn" title="Toggle Chapters (T)">&#9776; Chapters</button>
      <a href="index.html" class="toolbar-brand">SATYA: AWAKENED // 2035</a>
      <span class="toolbar-crumb">{esc_part_short} &middot; {esc_title}</span>
    </div>
    <div class="toolbar-cluster">
      <div class="toolbar-group" role="group" aria-label="Book Theme">
        <button type="button" class="toolbar-btn is-active" data-set-theme="awakened-lotus" title="Awakened 2035 indigo, blue lotus &amp; Faraday copper dossier">Blue Lotus</button>
        <button type="button" class="toolbar-btn" data-set-theme="sangha-robes" title="Korean Seon grey &amp; Japanese Zen black robes with subdued saffron borders">Sangha Robes</button>
        <button type="button" class="toolbar-btn" data-set-theme="chan-sanctuary">Chan Wood</button>
        <button type="button" class="toolbar-btn" data-set-theme="tactical-hud">Tactical HUD</button>
      </div>
      <div class="toolbar-group" role="group" aria-label="Page View">
        <button type="button" class="toolbar-btn is-active" data-set-view="spread">2-Page Spread</button>
        <button type="button" class="toolbar-btn" data-set-view="single">Single Page</button>
        <button type="button" class="toolbar-btn" data-set-view="scroll">Scroll</button>
      </div>
    </div>
    <div class="toolbar-cluster">
      <button id="btn-page-prev" type="button" class="toolbar-btn" title="Previous Page (Left Arrow)">&larr; Prev</button>
      <label class="page-jump-wrap" aria-label="Jump to Page">
        <span>Pg</span>
        <input id="book-page-input" type="number" min="1" value="1"/>
        <span id="book-page-total">/ 1</span>
      </label>
      <button id="btn-page-next" type="button" class="toolbar-btn" title="Next Page (Right Arrow)">Next &rarr;</button>
      <button id="btn-book-print" type="button" class="toolbar-btn" title="Print or Save PDF">Print / PDF</button>
    </div>
  </header>
  <div class="book-shell">
    <aside id="book-sidebar" class="book-sidebar">
      <div class="book-brand">SATYA: AWAKENED // 2035</div>
      <div class="book-sub">Earth 2035 Campaign Compendium</div>
      <a href="../book/index.html" class="book-cross-link">&larr; TTRPG Rules Book</a>
      {"".join(nav_items)}
    </aside>
    <main class="book-workspace">
      <div id="book-source" class="book-source-content">
        {body_html}
      </div>
      <div id="book-stage" class="book-stage" aria-live="polite"></div>
      <div class="book-pager">
        {"".join(footer_links)}
      </div>
    </main>
  </div>
</body>
</html>
"""


def build_static_campaign_book(
    output_dir: Path | None = None,
    repo_root: Path | None = None,
) -> dict[str, Any]:
    """Compile all 22 `docs/campaign/` chapters into static HTML pages and `catalog.json` inside `docs/campaign-book/`."""
    root = repo_root or _default_repo_root()
    out_dir = output_dir or (root / "docs" / "campaign-book")
    out_dir.mkdir(parents=True, exist_ok=True)

    assets_dir = root / "crates" / "satya-rules-book" / "assets" / "rules_book"
    if not assets_dir.exists():
        assets_dir = _default_repo_root() / "crates" / "satya-rules-book" / "assets" / "rules_book"
    asset_files: list[str] = []
    for asset_name in ("satya_book.css", "satya_book.js"):
        src_asset = assets_dir / asset_name
        dst_asset = out_dir / asset_name
        if src_asset.exists():
            dst_asset.write_text(src_asset.read_text(encoding="utf-8"), encoding="utf-8")
            asset_files.append(str(dst_asset))

    catalog_chapters: list[dict[str, Any]] = []
    generated_files: list[str] = []
    total_words = 0

    for idx, spec in enumerate(CAMPAIGN_CHAPTER_SPECS):
        cid = spec["chapter_id"]
        md = load_campaign_chapter_markdown_by_id(cid, repo_root=root)
        total_words += len(md.split())
        headings = extract_markdown_headings(md)
        rendered = _rewrite_campaign_links(render_markdown_to_html_python(md))

        prev_spec = CAMPAIGN_CHAPTER_SPECS[idx - 1] if idx > 0 else None
        next_spec = CAMPAIGN_CHAPTER_SPECS[idx + 1] if idx + 1 < len(CAMPAIGN_CHAPTER_SPECS) else None
        page_html = _wrap_campaign_page_html(
            title=spec["title"],
            active_chapter_id=cid,
            body_html=rendered,
            prev_spec=prev_spec,
            next_spec=next_spec,
        )
        chapter_file = out_dir / f"{cid}.html"
        chapter_file.write_text(page_html, encoding="utf-8")
        generated_files.append(str(chapter_file))

        catalog_chapters.append(
            {
                "chapter_id": cid,
                "part": spec["part"],
                "title": spec["title"],
                "relative_path": spec["relative_path"],
                "visibility": spec["visibility"],
                "description": spec["description"],
                "headings": headings,
                "html_file": f"{cid}.html",
            }
        )

    toc_sections: list[str] = []
    current_part = ""
    part_items: list[str] = []
    for idx, ch in enumerate(catalog_chapters, start=1):
        if ch["part"] != current_part:
            if part_items:
                toc_sections.append(f'<ul class="rules-list">{"".join(part_items)}</ul>')
                part_items = []
            current_part = ch["part"]
            toc_sections.append(f"<h3>{html.escape(current_part)}</h3>")
        badge = _visibility_badge_html(ch["visibility"], include_public=True)
        part_items.append(
            f'<li><strong><a href="{ch["html_file"]}">{idx}. {html.escape(ch["title"])}</a></strong> {badge} '
            f'<span class="rules-inline-code">docs/campaign/{html.escape(ch["relative_path"])}</span><br/>'
            f'{html.escape(ch["description"])}</li>'
        )
    if part_items:
        toc_sections.append(f'<ul class="rules-list">{"".join(part_items)}</ul>')

    summary_md_path = root / "docs" / "campaign" / "SUMMARY.md"
    summary_html = (
        _rewrite_campaign_links(render_markdown_to_html_python(summary_md_path.read_text(encoding="utf-8")))
        if summary_md_path.exists()
        else ""
    )

    index_body = f"""
    <h1>Satya: Awakened (2035 AD) — Campaign Compendium</h1>
    <p>
      Welcome to the static HTML Campaign Compendium generated from <code>docs/campaign/</code>.
      This 22-chapter lore bible pairs with the <a class="rules-link" href="../book/index.html">Satya TTRPG Rules Book</a>
      and mirrors the visibility gates (<code>PUBLIC</code>, <code>ARC2_SATYANG</code>, <code>GM_CONSPIRACY_SECRET</code>)
      enforced in the Web Client and Hybrid PyLance RAG index.
    </p>
    <h2>Chapter Directory</h2>
    {"".join(toc_sections)}
    {summary_html}
    """

    index_html = _wrap_campaign_page_html(
        title="Table of Contents & Spoiler Guide",
        active_chapter_id="index",
        body_html=index_body,
        prev_spec=None,
        next_spec=CAMPAIGN_CHAPTER_SPECS[0],
    )
    index_path = out_dir / "index.html"
    index_path.write_text(index_html, encoding="utf-8")
    generated_files.append(str(index_path))

    catalog_payload = {
        "book_title": "Satya: Awakened (Earth 2035 AD) Campaign Compendium",
        "using_satya_py": using_satya_py_backend(),
        "chapter_count": len(catalog_chapters),
        "chapters": catalog_chapters,
    }
    catalog_path = out_dir / "catalog.json"
    catalog_path.write_text(json.dumps(catalog_payload, indent=2), encoding="utf-8")
    generated_files.append(str(catalog_path))

    return {
        "output_dir": str(out_dir),
        "chapter_count": len(catalog_chapters),
        "total_words": total_words,
        "generated_files": generated_files,
        "asset_files": asset_files,
        "using_satya_py": using_satya_py_backend(),
    }


build_campaign_book = build_static_campaign_book


if __name__ == "__main__":
    summary = build_static_campaign_book()
    print(
        f"Built Satya Awakened Campaign Book ({summary['chapter_count']} chapters) in {summary['output_dir']} "
        f"(satya_py active: {summary['using_satya_py']})"
    )
