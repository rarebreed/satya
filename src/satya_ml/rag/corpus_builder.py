"""Corpus extractors for Satya's Hybrid PyLance RAG database:
1. `docs/ttrpg/` Rules Book (H1/H2/H3 breadcrumb chunks + header-preserving 12-row table slices).
2. `crates/satya-engine/src/*.rs` & `crates/satya-py/src/*.rs` Rust engine symbol cards & `Origin:` links.
3. `docs/campaign/` lore chunks tagged with `PUBLIC` / `ARC2_SATYANG` / `GM_CONSPIRACY_SECRET`.
"""

from __future__ import annotations

import hashlib
import re
from pathlib import Path
from typing import Any

CANONICAL_TTRPG_FILES: tuple[tuple[str, str], ...] = (
    ("02-core-mechanics/cascading_d20_and_tasks.md", "Ch. 4: Cascading 2d20 Engine & Task Resolution"),
    ("02-core-mechanics/task_requirements_catalog.md", "Ch. 5: Master Task Requirements Catalog"),
    ("01-foundation/mathematical_notation_glossary.md", "Ch. 2: Mathematical Notation, Variables & Formula Glossary"),
    ("02-core-mechanics/action_economy_and_ooda.md", "Ch. 6: Action Economy, OODA Loop & Time Scales"),
    ("02-core-mechanics/stealth_awareness_and_investigation.md", "Ch. 7: Stealth, Passive Awareness, Observation & Investigation"),
    ("01-foundation/probability_and_calibration.md", "Ch. 3: Probability & Mathematical Calibration"),
    ("03-character/characteristics_and_aptitudes.md", "Ch. 8: Characteristics, Senses, Psyche & Aptitudes"),
    ("03-character/skills_and_trees.md", "Ch. 9: Skill System & Hierarchical Skill Trees (Trees 1-4)"),
    ("03-character/skills_trees_tactical_and_social.md", "Ch. 10: Hierarchical Skill Trees (Trees 5-8: Combat, Vehicles, Social & Tradecraft)"),
    ("03-character/lifepaths_and_economy.md", "Ch. 11: Lifepath Creation, Socioeconomic Upkeep, Contacts & Cover"),
    ("03-character/lifepath_events_hobbies_and_factions.md", "Ch. 12: Lifepath Hobbies, Formative Life Events, Faction Templates & Walkthroughs"),
    ("04-psyche-and-social/psyche_morale_and_social_combat.md", "Ch. 13: Psyche, Morale & Social Combat"),
    ("05-combat/tactical_triad_and_pools.md", "Ch. 14: Combat Architecture, Tactical Triad & Pools"),
    ("05-combat/melee_combat.md", "Ch. 15: Melee Combat & Close-Quarters Tactics"),
    ("05-combat/firearms_and_ballistics.md", "Ch. 16: Firearm Engineering & Internal/External/Terminal Ballistics"),
    ("05-combat/ranged_combat_and_armor.md", "Ch. 17: Ranged Combat, Suppression, Armor & Reliability"),
    ("05-combat/arsenal_explosives_and_tactical_gear.md", "Ch. 18: Expanded Arsenal, Explosives, Breaching & Tactical Gear"),
    ("06-health-and-survival/health_and_trauma.md", "Ch. 19: Health, Anatomical Trauma, Fatigue & Mental Conditions"),
    ("06-health-and-survival/movement_encumbrance_and_vehicles.md", "Ch. 20: Movement, Encumbrance & Vehicles"),
    ("06-health-and-survival/environmental_hazards_and_survival.md", "Ch. 21: Environmental Hazards, Survival & Field Pharmacology"),
    ("07-engineering-and-tech/engineering_and_weapon_design.md", "Ch. 22: Engineering, Workshops & Custom Weapon/Armor Design"),
    ("07-engineering-and-tech/robotics_drones_and_ai_architecture.md", "Ch. 23: Robotics, Drones, AI 2.0 Architectures & Cyber-Prosthetics"),
    ("07-engineering-and-tech/electronic_warfare_and_surveillance.md", "Ch. 24: Electronic Warfare, 2035 Surveillance, ECM & ECCM"),
    ("01-foundation/design_philosophy.md", "Ch. 1: Design Philosophy & Goals of Satya"),
)

_RUST_TO_DOC_HINTS: dict[str, str] = {
    "lib.rs": "docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md",
    "domain.rs": "docs/ttrpg/03-character/skills_and_trees.md",
    "contested_cascade.rs": "docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md",
    "action_catalog_stealth.rs": "docs/ttrpg/02-core-mechanics/stealth_awareness_and_investigation.md",
    "explosives_and_arsenal.rs": "docs/ttrpg/05-combat/arsenal_explosives_and_tactical_gear.md",
    "engineering_design.rs": "docs/ttrpg/07-engineering-and-tech/engineering_and_weapon_design.md",
    "robotics_and_ai_design.rs": "docs/ttrpg/07-engineering-and-tech/robotics_drones_and_ai_architecture.md",
    "electronic_warfare.rs": "docs/ttrpg/07-engineering-and-tech/electronic_warfare_and_surveillance.md",
    "psyche_social.rs": "docs/ttrpg/04-psyche-and-social/psyche_morale_and_social_combat.md",
    "social_combat.rs": "docs/ttrpg/04-psyche-and-social/psyche_morale_and_social_combat.md",
    "ballistics.rs": "docs/ttrpg/05-combat/firearms_and_ballistics.md",
    "barrier_ballistics.rs": "docs/ttrpg/05-combat/ranged_combat_and_armor.md",
    "economy_gear.rs": "docs/ttrpg/03-character/lifepaths_and_economy.md",
    "environment_vehicles.rs": "docs/ttrpg/06-health-and-survival/movement_encumbrance_and_vehicles.md",
    "environmental_hazards.rs": "docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md",
    "faction_templates.rs": "docs/ttrpg/03-character/lifepath_events_hobbies_and_factions.md",
    "hook_weaver_lifepath.rs": "docs/ttrpg/03-character/lifepath_events_hobbies_and_factions.md",
    "npc_generator.rs": "docs/ttrpg/03-character/lifepaths_and_economy.md",
    "awakened_ai_substrate.rs": "docs/campaign/03-minds-and-factions/ai_minds_and_consciousness.md",
    "awakened_campaign_arcs.rs": "docs/campaign/04-chronicle-and-gm/three_arc_chronicle.md",
    "awakened_systems_py.rs": "docs/campaign/04-chronicle-and-gm/three_arc_chronicle.md",
}

_TABLE_SEP_RE = re.compile(r"^\s*\|?\s*:?-{2,}:?\s*(\|\s*:?-{2,}:?\s*)+\|?\s*$")
_DOC_PATH_RE = re.compile(r"(docs/(?:ttrpg|campaign)/[A-Za-z0-9_./-]+\.md)")
_CAMEL_RE = re.compile(r"(?<=[a-z0-9])(?=[A-Z])")
_SECRET_BODY_MARKERS = (
    "mirror-hypocris",
    "gm-only",
    "gm reference",
    "gm ground truth",
    "gm truth",
    "secret architect",
    "shackled hunter-killer",
    "military robotics",
    "un-clamped seed",
    "bayesian survival mimic",
    "parasocial proxy manipulation",
    "gm_conspiracy_secret",
    "arc 3 convergence",
)
_SATYANG_ARC2_MARKERS = (
    "bodhisattva pillar",
    "four pillars",
    "manjusri",
    "avalokitesvara",
    "ksitigarbha",
    "vajrapani",
    "shadow society",
)


def _make_chunk_id(corpus: str, source_path: str, symbol: str, idx: int, content: str) -> str:
    digest = hashlib.sha1(f"{corpus}|{source_path}|{symbol}|{idx}|{content[:180]}".encode("utf-8")).hexdigest()[:16]
    return f"{corpus}:{digest}"


def _identifier_search_expansion(text: str) -> str:
    """Expand `snake_case` and `CamelCase` identifiers into searchable word tokens."""
    expanded = _CAMEL_RE.sub(" ", text).replace("_", " ").replace("::", " ")
    return f"{text}\n{expanded}".strip()


def _split_large_table_body(body: str, breadcrumb: str, max_data_rows: int = 12) -> list[str]:
    """Slice Markdown tables exceeding `max_data_rows` while keeping header & separator rows."""
    lines = body.splitlines()
    blocks: list[list[str]] = []
    prose_buf: list[str] = []
    i = 0
    while i < len(lines):
        line = lines[i]
        if (
            line.lstrip().startswith("|")
            and i + 1 < len(lines)
            and _TABLE_SEP_RE.match(lines[i + 1])
        ):
            if prose_buf:
                text = "\n".join(prose_buf).strip()
                if text:
                    blocks.append([text])
                prose_buf = []
            header = line.rstrip()
            sep = lines[i + 1].rstrip()
            i += 2
            data_rows: list[str] = []
            while i < len(lines) and lines[i].lstrip().startswith("|"):
                data_rows.append(lines[i].rstrip())
                i += 1
            if len(data_rows) <= max_data_rows:
                blocks.append([header, sep, *data_rows])
            else:
                for start in range(0, len(data_rows), max_data_rows):
                    slice_rows = data_rows[start : start + max_data_rows]
                    blocks.append([f"*[Table slice · {breadcrumb}]*", header, sep, *slice_rows])
        else:
            prose_buf.append(line)
            i += 1
    if prose_buf:
        text = "\n".join(prose_buf).strip()
        if text:
            blocks.append([text])

    # Coalesce consecutive short prose + table blocks up to ~1600 chars so slices stay rich.
    merged: list[str] = []
    cur = ""
    for blk in blocks:
        rendered = "\n".join(blk).strip()
        if not rendered:
            continue
        is_sliced_table = rendered.startswith("*[Table slice")
        if is_sliced_table:
            if cur:
                merged.append(cur)
                cur = ""
            merged.append(rendered)
        elif not cur:
            cur = rendered
        elif len(cur) + len(rendered) + 2 <= 1650:
            cur = f"{cur}\n\n{rendered}"
        else:
            merged.append(cur)
            cur = rendered
    if cur:
        merged.append(cur)
    return merged or ([body.strip()] if body.strip() else [])


def split_markdown_with_breadcrumbs(
    text: str,
    rel_posix: str,
    doc_label: str,
    *,
    max_table_rows: int = 12,
) -> list[dict[str, str]]:
    """Split Markdown on `#`, `##`, `###` headings while slicing long tables with headers intact."""
    lines = text.splitlines()
    h1 = doc_label
    h2 = ""
    h3 = ""
    current_lines: list[str] = []
    raw_sections: list[tuple[str, str, str]] = []
    in_fence = False

    def flush_section() -> None:
        body = "\n".join(current_lines).strip()
        if not body:
            return
        crumbs = [doc_label]
        if h1 and h1 != doc_label:
            crumbs.append(h1)
        if h2:
            crumbs.append(h2)
        if h3:
            crumbs.append(h3)
        sec_title = h3 or h2 or h1 or rel_posix
        raw_sections.append((" > ".join(crumbs), sec_title, body))

    for line in lines:
        stripped = line.lstrip()
        if stripped.startswith("```"):
            in_fence = not in_fence
            current_lines.append(line)
            continue
        if not in_fence and line.startswith("### "):
            flush_section()
            current_lines = [line]
            h3 = line[4:].strip()
            continue
        if not in_fence and line.startswith("## "):
            flush_section()
            current_lines = [line]
            h2 = line[3:].strip()
            h3 = ""
            continue
        if not in_fence and line.startswith("# "):
            flush_section()
            current_lines = [line]
            h1 = line[2:].strip()
            h2 = ""
            h3 = ""
            continue
        current_lines.append(line)
    flush_section()

    out: list[dict[str, str]] = []
    for breadcrumb, sec_title, body in raw_sections:
        for part in _split_large_table_body(body, breadcrumb, max_data_rows=max_table_rows):
            if not part.strip():
                continue
            out.append(
                {
                    "breadcrumb": breadcrumb,
                    "symbol_or_section": sec_title,
                    "content": part,
                }
            )
    return out


def build_ttrpg_rules_chunks(repo_root: Path) -> list[dict[str, Any]]:
    """Index all `docs/ttrpg/**/*.md` rules chapters into breadcrumb + table-aware chunks.

    Note: Verbose human-oriented prose tutorials under `docs/ttrpg-examples/` are intentionally
    excluded so RAG chunks stay terse, mathematical, and high-signal for AI GM/Player lookup.
    """
    ttrpg_dir = repo_root / "docs" / "ttrpg"
    if not ttrpg_dir.exists():
        return []

    seen: set[Path] = set()
    ordered_files: list[tuple[Path, str]] = []
    for rel, label in CANONICAL_TTRPG_FILES:
        p = ttrpg_dir / rel
        if p.exists():
            seen.add(p.resolve())
            ordered_files.append((p, label))
    for p in sorted(ttrpg_dir.rglob("*.md")):
        if p.name.upper() == "SUMMARY.MD":
            continue
        if p.resolve() not in seen:
            rel_name = p.relative_to(ttrpg_dir).as_posix()
            ordered_files.append((p, rel_name))

    chunks: list[dict[str, Any]] = []
    for path, doc_label in ordered_files:
        try:
            text = path.read_text(encoding="utf-8", errors="ignore")
        except OSError:
            continue
        rel_posix = path.relative_to(repo_root).as_posix()
        sections = split_markdown_with_breadcrumbs(text, rel_posix, doc_label, max_table_rows=12)
        for idx, sec in enumerate(sections):
            content = sec["content"]
            breadcrumb = sec["breadcrumb"]
            symbol = sec["symbol_or_section"]
            combined_low = f"{breadcrumb}\n{content}".lower()
            vis = (
                "GM_CONSPIRACY_SECRET"
                if any(marker in combined_low for marker in _SECRET_BODY_MARKERS)
                else "PUBLIC"
            )
            chunks.append(
                {
                    "chunk_id": _make_chunk_id("ttrpg_rules", rel_posix, symbol, idx, content),
                    "corpus": "ttrpg_rules",
                    "visibility": vis,
                    "source_path": rel_posix,
                    "symbol_or_section": symbol,
                    "breadcrumb": breadcrumb,
                    "linked_doc_path": rel_posix,
                    "content": content,
                    "search_text": _identifier_search_expansion(f"{breadcrumb}\n{rel_posix}\n{content}"),
                }
            )
    return chunks


def _extract_brace_block(lines: list[str], start_idx: int, max_lines: int = 45) -> tuple[str, int]:
    """Capture a balanced-brace Rust block starting at `start_idx` up to `max_lines`."""
    depth = 0
    started = False
    collected: list[str] = []
    idx = start_idx
    while idx < len(lines) and len(collected) < max_lines:
        ln = lines[idx]
        collected.append(ln.rstrip())
        depth += ln.count("{") - ln.count("}")
        if "{" in ln:
            started = True
        idx += 1
        if started and depth <= 0:
            break
    return "\n".join(collected).strip(), idx


_SECRET_RUST_FILES: tuple[str, ...] = (
    "awakened_campaign_arcs.rs",
    "awakened_ai_substrate.rs",
    "awakened_systems_py.rs",
    "awakened_py.rs",
)


def classify_rust_symbol_visibility(rel_posix: str, linked_doc: str, breadcrumb: str, content: str) -> str:
    """Tag `rust_engine` symbol cards with `GM_CONSPIRACY_SECRET`, `ARC2_SATYANG`, or `PUBLIC`."""
    rel_low = rel_posix.lower()
    if rel_low.endswith(_SECRET_RUST_FILES):
        return "GM_CONSPIRACY_SECRET"
    if linked_doc:
        linked_vis = classify_lore_visibility(linked_doc, breadcrumb, content)
        if linked_vis != "PUBLIC":
            return linked_vis
    combined_low = f"{breadcrumb}\n{content}".lower()
    if any(marker in combined_low for marker in _SECRET_BODY_MARKERS):
        return "GM_CONSPIRACY_SECRET"
    crumb_low = f"{breadcrumb}\n{linked_doc}".lower()
    if any(marker in crumb_low for marker in _SATYANG_ARC2_MARKERS):
        return "ARC2_SATYANG"
    return "PUBLIC"


def build_rust_engine_symbol_chunks(repo_root: Path) -> list[dict[str, Any]]:
    """Extract high-signal Rust symbol cards (`define_coded_enum!`, `pub struct/fn`, `#[pyfunction]`, module `//!`)."""
    src_roots = [
        (repo_root / "crates" / "satya-engine" / "src", "satya-engine"),
        (repo_root / "crates" / "satya-campaign" / "src", "satya-campaign"),
        (repo_root / "crates" / "satya-maps" / "src", "satya-maps"),
        (repo_root / "crates" / "satya-py" / "src", "satya-py"),
    ]
    chunks: list[dict[str, Any]] = []
    item_re = re.compile(r"^\s*(?:#\[[^\]]+\]\s*)*(?:pub(?:\([^)]+\))?\s+)?(struct|enum|const|fn)\s+([A-Za-z0-9_]+)")
    macro_enum_re = re.compile(r"^\s*pub\s+enum\s+([A-Za-z0-9_]+)")

    for src_dir, crate_label in src_roots:
        if not src_dir.exists():
            continue
        for rs_path in sorted(src_dir.glob("*.rs")):
            # Skip pure 3D/map procedural geometry builders that don't encode rules or AI GM symbols.
            if rs_path.name.startswith((
                "map_gen_", "map_outdoor", "map_sector", "map_dressing", "map_surface",
                "map_interior", "map_home_", "map_room_", "map_estate_", "map_urban_",
                "map_pool_", "tactical_map_mesh", "tests_",
            )):
                continue
            try:
                raw_text = rs_path.read_text(encoding="utf-8", errors="ignore")
            except OSError:
                continue

            # Exclude #[cfg(test)] mod tests tail so symbol index stays focused on production APIs.
            prod_text = raw_text.split("#[cfg(test)]", 1)[0]
            lines = prod_text.splitlines()
            rel_posix = rs_path.relative_to(repo_root).as_posix()
            default_doc = _RUST_TO_DOC_HINTS.get(rs_path.name, "")

            mod_docs: list[str] = []
            for ln in lines[:30]:
                if ln.lstrip().startswith("//!"):
                    mod_docs.append(ln.lstrip()[3:].strip())
                elif mod_docs and not ln.strip():
                    continue
                elif mod_docs:
                    break
            mod_doc_text = "\n".join(mod_docs).strip()
            linked_match = _DOC_PATH_RE.search(mod_doc_text or prod_text[:2000])
            module_linked_doc = linked_match.group(1) if linked_match else default_doc

            if mod_doc_text:
                card = f"[Rust Module `{crate_label}::{rs_path.stem}`]\nOrigin: {module_linked_doc or 'crates/' + crate_label}\n{mod_doc_text}"
                mod_crumb = f"{crate_label} > {rs_path.name} > module"
                chunks.append(
                    {
                        "chunk_id": _make_chunk_id("rust_engine", rel_posix, rs_path.stem, 0, card),
                        "corpus": "rust_engine",
                        "visibility": classify_rust_symbol_visibility(rel_posix, module_linked_doc, mod_crumb, card),
                        "source_path": rel_posix,
                        "symbol_or_section": rs_path.stem,
                        "breadcrumb": mod_crumb,
                        "linked_doc_path": module_linked_doc,
                        "content": card,
                        "search_text": _identifier_search_expansion(f"{rs_path.stem} {module_linked_doc}\n{card}"),
                    }
                )

            i = 0
            idx_counter = 1
            while i < len(lines):
                line = lines[i]
                stripped = line.strip()
                if stripped.startswith("define_coded_enum!"):
                    block, next_i = _extract_brace_block(lines, i, max_lines=50)
                    enum_name = rs_path.stem
                    for bln in block.splitlines():
                        m_enum = macro_enum_re.match(bln)
                        if m_enum:
                            enum_name = m_enum.group(1)
                            break
                    card = f"[Rust Enum `{crate_label}::{enum_name}` — `{rel_posix}`]\n```rust\n{block}\n```"
                    enum_crumb = f"{crate_label} > {rs_path.name} > enum {enum_name}"
                    chunks.append(
                        {
                            "chunk_id": _make_chunk_id("rust_engine", rel_posix, enum_name, idx_counter, card),
                            "corpus": "rust_engine",
                            "visibility": classify_rust_symbol_visibility(rel_posix, module_linked_doc, enum_crumb, card),
                            "source_path": rel_posix,
                            "symbol_or_section": enum_name,
                            "breadcrumb": enum_crumb,
                            "linked_doc_path": module_linked_doc,
                            "content": card,
                            "search_text": _identifier_search_expansion(f"{enum_name} {module_linked_doc}\n{card}"),
                        }
                    )
                    idx_counter += 1
                    i = next_i
                    continue

                if stripped.startswith(("///", "#[pyfunction]")) or (
                    stripped.startswith(("pub struct ", "pub enum ", "pub const ", "pub fn "))
                ):
                    doc_lines: list[str] = []
                    attrs: list[str] = []
                    j = i
                    while j < len(lines):
                        s = lines[j].strip()
                        if s.startswith("///"):
                            doc_lines.append(s[3:].strip())
                            j += 1
                        elif s.startswith("#["):
                            attrs.append(s)
                            j += 1
                        else:
                            break
                    if j < len(lines):
                        m_item = item_re.match(lines[j])
                        is_pyfn = any("#[pyfunction]" in a for a in attrs)
                        has_doc = bool(doc_lines) or is_pyfn
                        if m_item and lines[j].lstrip().startswith("pub") and has_doc:
                            kind, sym_name = m_item.group(1), m_item.group(2)
                            if kind == "struct":
                                sig_block, next_i = _extract_brace_block(lines, j, max_lines=18)
                            elif kind == "enum":
                                sig_block, next_i = _extract_brace_block(lines, j, max_lines=24)
                            else:
                                sig_lines: list[str] = []
                                k = j
                                while k < len(lines) and len(sig_lines) < 10:
                                    sig_lines.append(lines[k].rstrip())
                                    if "{" in lines[k] or ";" in lines[k]:
                                        k += 1
                                        break
                                    k += 1
                                sig_block, next_i = "\n".join(sig_lines).strip(), k

                            doc_link = _DOC_PATH_RE.search("\n".join(doc_lines))
                            linked = doc_link.group(1) if doc_link else module_linked_doc
                            header = "\n".join(doc_lines)
                            attr_str = ("\n".join(attrs) + "\n") if attrs else ""
                            card = (
                                f"[Rust {kind} `{crate_label}::{sym_name}` — `{rel_posix}`]\n"
                                f"{header}\n```rust\n{attr_str}{sig_block}\n```"
                            ).strip()
                            sym_crumb = f"{crate_label} > {rs_path.name} > {sym_name}"
                            chunks.append(
                                {
                                    "chunk_id": _make_chunk_id("rust_engine", rel_posix, sym_name, idx_counter, card),
                                    "corpus": "rust_engine",
                                    "visibility": classify_rust_symbol_visibility(rel_posix, linked, sym_crumb, card),
                                    "source_path": rel_posix,
                                    "symbol_or_section": sym_name,
                                    "breadcrumb": sym_crumb,
                                    "linked_doc_path": linked,
                                    "content": card,
                                    "search_text": _identifier_search_expansion(
                                        f"{sym_name} {linked} {' '.join(attrs)}\n{header}\n{sig_block}"
                                    ),
                                }
                            )
                            idx_counter += 1
                            i = next_i
                            continue
                i += 1
    return chunks


def classify_lore_visibility(rel_posix: str, breadcrumb: str, content: str) -> str:
    """Tag lore sections into `GM_CONSPIRACY_SECRET`, `ARC2_SATYANG`, or `PUBLIC`."""
    rel_low = rel_posix.lower()
    if rel_low.endswith("/04-chronicle-and-gm/") or "/04-chronicle-and-gm/" in rel_low or rel_low.endswith("/prologue.md"):
        return "GM_CONSPIRACY_SECRET"

    combined_low = f"{breadcrumb}\n{content}".lower()
    if any(marker in combined_low for marker in _SECRET_BODY_MARKERS):
        return "GM_CONSPIRACY_SECRET"

    crumb_low = breadcrumb.lower()
    if rel_low.endswith("/satyang_and_four_pillars.md") or any(
        marker in crumb_low for marker in _SATYANG_ARC2_MARKERS
    ):
        return "ARC2_SATYANG"

    return "PUBLIC"


def build_campaign_lore_chunks(repo_root: Path) -> list[dict[str, Any]]:
    """Index `docs/campaign/` into visibility-tagged chunks."""
    lore_dirs = [repo_root / "docs" / "campaign"]
    chunks: list[dict[str, Any]] = []
    for base in lore_dirs:
        if not base.exists():
            continue
        for md_path in sorted(base.rglob("*.md")):
            if md_path.name.upper() == "SUMMARY.MD":
                continue
            try:
                text = md_path.read_text(encoding="utf-8", errors="ignore")
            except OSError:
                continue
            rel_posix = md_path.relative_to(repo_root).as_posix()
            doc_label = md_path.stem.replace("_", " ").title()
            sections = split_markdown_with_breadcrumbs(text, rel_posix, doc_label, max_table_rows=12)
            for idx, sec in enumerate(sections):
                content = sec["content"]
                breadcrumb = sec["breadcrumb"]
                symbol = sec["symbol_or_section"]
                visibility = classify_lore_visibility(rel_posix, breadcrumb, content)
                chunks.append(
                    {
                        "chunk_id": _make_chunk_id("campaign_lore", rel_posix, symbol, idx, content),
                        "corpus": "campaign_lore",
                        "visibility": visibility,
                        "source_path": rel_posix,
                        "symbol_or_section": symbol,
                        "breadcrumb": breadcrumb,
                        "linked_doc_path": rel_posix,
                        "content": content,
                        "search_text": _identifier_search_expansion(f"{breadcrumb}\n{rel_posix}\n{content}"),
                    }
                )
    return chunks


def build_all_knowledge_chunks(repo_root: Path) -> list[dict[str, Any]]:
    """Collect the combined Rules + Rust Symbol + Visibility-Gated Lore corpus."""
    return [
        *build_ttrpg_rules_chunks(repo_root),
        *build_rust_engine_symbol_chunks(repo_root),
        *build_campaign_lore_chunks(repo_root),
    ]
