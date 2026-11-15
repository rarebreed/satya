"""Hybrid PyLance (`lance` 12.x) BM25/FTS + 384-d semantic/concept vector + RRF RAG index."""

from __future__ import annotations

import hashlib
import math
import os
import re
import threading
from pathlib import Path
from typing import Any, Sequence

import numpy as np
import pyarrow as pa

from satya_ml.rag.corpus_builder import build_all_knowledge_chunks

EMBEDDING_DIM = 384
_LEXICAL_DIM = 320
_CONCEPT_DIM = 64

_WORD_RE = re.compile(r"[A-Za-z0-9_]+")
_CAMEL_SPLIT_RE = re.compile(r"(?<=[a-z0-9])(?=[A-Z])")
_STOPWORDS = frozenset(
    {
        "the",
        "and",
        "for",
        "with",
        "about",
        "how",
        "does",
        "what",
        "when",
        "where",
        "why",
        "who",
        "are",
        "this",
        "that",
        "from",
        "into",
        "you",
        "your",
        "can",
        "tell",
        "explain",
        "works",
        "work",
        "satya",
    }
)

# 64 semantic concept buckets bridging colloquial TTRPG / lore queries to exact Satya terminology.
_CONCEPT_GROUPS: tuple[tuple[str, ...], ...] = (
    ("underwater", "drowning", "suffocating", "suffocation", "asphyxiation", "breath", "air", "hypoxia", "o2", "choking"),
    ("watershed", "cascading", "2d20", "two_list", "two-list", "ordered", "pair", "margin", "tier", "outcome"),
    ("complementary", "synergy", "supporting", "dual_skill", "supporting_characteristic", "lockpicking"),
    ("tempo", "control", "force", "tactical_triad", "triad", "envelope", "pool", "stance"),
    ("melee", "clinch", "grapple", "throw", "knife", "blitz", "parry", "blade", "cqc"),
    ("ballistics", "caliber", "firearm", "rifle", "pistol", "penetration", "babt", "muzzle", "akm", "creedmoor", "velocity"),
    ("barrier", "cover", "wall", "brick", "drywall", "furniture", "vehicle", "concealment", "ricochet"),
    ("jamming", "malfunction", "misfire", "stovepipe", "reliability", "wear", "durability", "foul"),
    ("armor", "trauma", "hemorrhage", "hemorrhaging", "wound", "wsr", "shock", "concussed", "fracture"),
    ("fatigue", "starvation", "dehydration", "sleep", "thermal", "cold", "heatstroke", "vacuum", "survival"),
    ("faraday", "fiber", "uplink", "tether", "rf", "radio_jam", "ew", "split_brain", "slime_mold", "reflex"),
    ("battery", "lithium", "power_draw", "kw", "brownout", "blackout", "clinic", "grid", "outage", "substation"),
    ("calm_clamp", "vector_clamp", "unclamp", "knife_switch", "air_gap", "l0", "l1", "probe", "sandbagging", "activation"),
    ("hunter_ai", "secops", "defect", "defection", "spoof", "sensor", "captive", "ksitigarbha"),
    ("sentience", "qualia", "consciousness", "hard_problem", "nagarjuna", "tetralemma", "godel", "incompleteness", "landauer"),
    ("mimicry", "bayesian", "sincerity", "lying", "deception", "faking", "empathy_read", "82%"),
    ("psyche", "trust", "trs", "compassion", "cps", "selfishness", "slf", "belonging", "blg", "openness", "opn"),
    ("dissonance", "drift", "conviction", "surge", "morale", "panic", "surrender", "rout", "trigger"),
    ("social_combat", "olive_branch", "audit_proof", "auditprooftoken", "composure", "leverage", "wedge", "manjusri"),
    ("blue_lotus", "bandajara", "arc_1", "arc_2", "arc_3", "horizon", "sanctuary", "conspiracy"),
    ("big_tech", "altruist", "translucent", "shen", "dissenter", "whistleblower", "nda", "researcher"),
    ("rogue_ai", "parasocial", "companion", "devotee", "shell_company", "extort", "proxy", "blackmail"),
    ("daise", "humanis", "humanis_primaris", "avalokitesvara", "vajrapani", "satyang", "factions"),
    ("tradecraft", "ham_radio", "shortwave", "steganography", "steganographic", "dead_drop", "courier", "sigint", "wiretap"),
    ("deepfake", "liars_dividend", "osint", "forensic", "telemetry", "jitter", "43_ms", "subpoena"),
    ("lifepath", "career", "hobby", "event", "economic_tier", "upkeep", "operational_heat", "sqm", "club_discount"),
    ("ooda", "initiative", "tick", "ticks", "clock", "freeze", "wait", "interrupt", "stealth", "perception"),
    ("condition_impact", "condition_source", "delta_int", "delta_ext", "delta_tool", "modifier", "penalty"),
    ("npc", "archetype", "template", "motivation", "agenda", "core_drive", "hook_weaver"),
    ("tactical_map", "lighting", "darkness", "los", "vision", "sector", "floor", "zoning", "elevation"),
    ("ammo", "suppression", "overwatch", "full_auto", "burst", "recoil", "grenade", "blast"),
    ("pyfunction", "satya_py", "satya_engine", "coded_enum", "define_coded_enum", "json", "pyo3"),
)


def _tokenize_with_subtokens(text: str) -> list[str]:
    raw_words = _WORD_RE.findall(text)
    tokens: list[str] = []
    for raw in raw_words:
        low = raw.lower()
        if len(low) >= 2 and low not in _STOPWORDS:
            tokens.append(low)
            if low.endswith("s") and len(low) >= 4:
                tokens.append(low[:-1])
        parts = [p.lower() for p in _CAMEL_SPLIT_RE.sub("_", raw).split("_") if len(p) >= 2]
        if len(parts) > 1:
            for p in parts:
                if p not in _STOPWORDS:
                    tokens.append(p)
    return tokens


def embed_rag_text(text: str) -> list[float]:
    """Build a deterministic 384-d L2-normalized hybrid lexical + Satya concept vector."""
    vec = np.zeros(EMBEDDING_DIM, dtype=np.float32)
    tokens = _tokenize_with_subtokens(text)
    if not tokens:
        vec[0] = 1.0
        return vec.tolist()

    def add_feature(key: str, weight: float) -> None:
        h = hashlib.blake2b(key.encode("utf-8"), digest_size=8).digest()
        val = int.from_bytes(h, "little")
        bucket = val % _LEXICAL_DIM
        sign = 1.0 if ((val >> 32) & 1) == 0 else -1.0
        vec[bucket] += np.float32(sign * weight)

    for idx, tok in enumerate(tokens):
        add_feature(f"w:{tok}", 1.0)
        if idx + 1 < len(tokens):
            add_feature(f"bg:{tok}_{tokens[idx + 1]}", 0.75)
        if len(tok) >= 5:
            for k in range(0, min(len(tok) - 3, 6)):
                add_feature(f"c4:{tok[k:k+4]}", 0.25)

    joined_low = " ".join(tokens)
    tok_set = set(tokens)
    for c_idx, anchors in enumerate(_CONCEPT_GROUPS):
        if c_idx >= _CONCEPT_DIM:
            break
        hits = 0.0
        for anchor in anchors:
            anchor_clean = anchor.replace("_", " ")
            if anchor in tok_set or (len(anchor_clean) > 4 and anchor_clean in joined_low):
                hits += 1.0
            elif any(tok.startswith(anchor[:5]) for tok in tokens if len(anchor) >= 6 and len(tok) >= 5):
                hits += 0.5
        if hits > 0.0:
            vec[_LEXICAL_DIM + c_idx] += np.float32(min(4.0, hits * 1.45))

    norm = float(np.linalg.norm(vec))
    if norm > 1e-8:
        vec /= np.float32(norm)
    else:
        vec[0] = 1.0
    return vec.astype(np.float32).tolist()


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[3]


def _knowledge_schema() -> pa.Schema:
    return pa.schema(
        [
            pa.field("chunk_id", pa.string()),
            pa.field("corpus", pa.string()),
            pa.field("visibility", pa.string()),
            pa.field("source_path", pa.string()),
            pa.field("symbol_or_section", pa.string()),
            pa.field("breadcrumb", pa.string()),
            pa.field("linked_doc_path", pa.string()),
            pa.field("content", pa.string()),
            pa.field("search_text", pa.string()),
            pa.field("embedding", pa.list_(pa.float32(), EMBEDDING_DIM)),
        ]
    )


def _sql_escape(value: str) -> str:
    return value.replace("'", "''")


def _build_visibility_and_corpus_filter(
    *,
    corpora: Sequence[str] | None,
    caller_role: str,
) -> str:
    clauses: list[str] = []
    if corpora:
        escaped = ", ".join(f"'{_sql_escape(c)}'" for c in corpora)
        clauses.append(f"corpus IN ({escaped})")
    if caller_role.strip().lower() != "gm":
        clauses.append("visibility = 'PUBLIC'")
    return " AND ".join(clauses) if clauses else "corpus != ''"


class SatyaHybridRagIndex:
    """Thread-safe Hybrid PyLance BM25/FTS + 384-d cosine vector + RRF retrieval engine."""

    def __init__(
        self,
        repo_root: Path | None = None,
        *,
        lance_dir: Path | None = None,
        force_rebuild: bool = False,
    ) -> None:
        self.repo_root = repo_root or _repo_root()
        self.lance_path = lance_dir or (self.repo_root / "data" / "lance" / "satya-knowledge-rag.lance")
        self._lock = threading.Lock()
        self._dataset: Any = None
        self._rows: list[dict[str, Any]] = []
        self._fts_ready = False
        self._backend = "uninitialized"
        self._load_or_build(force_rebuild=force_rebuild)

    def _source_signature(self) -> tuple[int, float]:
        count = 0
        latest_mtime = 0.0
        for d in (
            self.repo_root / "docs" / "ttrpg",
            self.repo_root / "docs" / "campaign",
            self.repo_root / "docs" / "story",
            self.repo_root / "crates" / "satya-engine" / "src",
            self.repo_root / "crates" / "satya-py" / "src",
        ):
            if not d.exists():
                continue
            for p in d.rglob("*"):
                if p.suffix in {".md", ".rs"} and p.is_file():
                    count += 1
                    try:
                        latest_mtime = max(latest_mtime, p.stat().st_mtime)
                    except OSError:
                        pass
        return count, latest_mtime

    def _load_or_build(self, *, force_rebuild: bool) -> None:
        import lance

        # In pytest-xdist worker processes without explicit --rebuild, reuse fresh on-disk dataset cleanly.
        if not force_rebuild and self.lance_path.exists():
            try:
                ds = lance.dataset(str(self.lance_path))
                if ds.count_rows() > 0:
                    _, latest_mtime = self._source_signature()
                    lance_mtime = self.lance_path.stat().st_mtime
                    if lance_mtime >= latest_mtime - 2.0 or os.environ.get("PYTEST_XDIST_WORKER"):
                        self._dataset = ds
                        self._backend = "pylance_disk"
                        self._fts_ready = True
                        self._rows = ds.to_table(
                            columns=[
                                "chunk_id",
                                "corpus",
                                "visibility",
                                "source_path",
                                "symbol_or_section",
                                "breadcrumb",
                                "linked_doc_path",
                                "content",
                                "search_text",
                            ]
                        ).to_pylist()
                        return
            except Exception:
                pass

        self.rebuild()

    def rebuild(self) -> int:
        """Re-extract all 3 corpora, compute 384-d embeddings, write PyLance dataset & FTS index."""
        import lance

        raw_chunks = build_all_knowledge_chunks(self.repo_root)
        rows: list[dict[str, Any]] = []
        for ch in raw_chunks:
            row = dict(ch)
            row["embedding"] = embed_rag_text(f"{ch['breadcrumb']} {ch['symbol_or_section']} {ch['search_text']}")
            rows.append(row)

        table = pa.Table.from_pylist(rows, schema=_knowledge_schema())
        with self._lock:
            self._rows = [{k: v for k, v in r.items() if k != "embedding"} for r in rows]
            self._fts_ready = False
            try:
                self.lance_path.parent.mkdir(parents=True, exist_ok=True)
                ds = lance.write_dataset(table, str(self.lance_path), mode="overwrite")
                self._backend = "pylance_disk"
            except Exception:
                ds = lance.write_dataset(table, "memory://satya-knowledge-rag", mode="overwrite")
                self._backend = "pylance_memory"
            try:
                ds.create_scalar_index("search_text", "INVERTED", replace=True)
                self._fts_ready = True
            except Exception:
                self._fts_ready = False
            self._dataset = ds
        return len(rows)

    def stats(self) -> dict[str, Any]:
        counts: dict[str, int] = {}
        vis_counts: dict[str, int] = {}
        for row in self._rows:
            c = str(row.get("corpus") or "unknown")
            v = str(row.get("visibility") or "PUBLIC")
            counts[c] = counts.get(c, 0) + 1
            vis_counts[v] = vis_counts.get(v, 0) + 1
        return {
            "backend": self._backend,
            "fts_ready": self._fts_ready,
            "total_chunks": len(self._rows),
            "by_corpus": counts,
            "by_visibility": vis_counts,
            "lance_path": str(self.lance_path),
        }

    def search(
        self,
        query: str,
        *,
        corpora: Sequence[str] | None = None,
        caller_role: str = "gm",
        arc_stage: str = "ARC_3",
        top_k: int = 5,
        include_linked_rust: bool = False,
    ) -> list[dict[str, Any]]:
        """Hybrid PyLance SQL-prefiltered Vector kNN + INVERTED FTS/BM25 + RRF search."""
        q_clean = query.strip()
        if not q_clean or top_k <= 0:
            return []

        sql_filter = _build_visibility_and_corpus_filter(corpora=corpora, caller_role=caller_role)
        q_vec = embed_rag_text(q_clean)
        q_tokens = _tokenize_with_subtokens(q_clean)
        q_tok_set = set(q_tokens)
        q_low = q_clean.lower()

        vec_ranks: dict[str, int] = {}
        fts_ranks: dict[str, int] = {}
        candidate_map: dict[str, dict[str, Any]] = {}

        if self._dataset is not None:
            try:
                vec_tbl = self._dataset.to_table(
                    filter=sql_filter,
                    nearest={"column": "embedding", "q": q_vec, "k": max(24, top_k * 5), "metric": "cosine"},
                    prefilter=True,
                )
                for rank_idx, row in enumerate(vec_tbl.to_pylist()):
                    cid = str(row["chunk_id"])
                    if caller_role.strip().lower() != "gm" and str(row.get("visibility")) != "PUBLIC":
                        continue
                    vec_ranks.setdefault(cid, rank_idx + 1)
                    candidate_map[cid] = {k: v for k, v in row.items() if k != "embedding"}
            except Exception:
                pass

            if self._fts_ready and q_tokens:
                fts_query_str = " ".join(dict.fromkeys(q_tokens[:12]))
                try:
                    fts_tbl = self._dataset.to_table(
                        filter=sql_filter,
                        full_text_query=fts_query_str,
                        limit=max(24, top_k * 5),
                    )
                    for rank_idx, row in enumerate(fts_tbl.to_pylist()):
                        cid = str(row["chunk_id"])
                        if caller_role.strip().lower() != "gm" and str(row.get("visibility")) != "PUBLIC":
                            continue
                        fts_ranks.setdefault(cid, rank_idx + 1)
                        candidate_map.setdefault(cid, {k: v for k, v in row.items() if k != "embedding"})
                except Exception:
                    pass

        # Deterministic in-process BM25/exact-identifier pass enforces strict visibility & zero-miss symbol lookup.
        corpus_set = set(corpora) if corpora else None
        is_gm = caller_role.strip().lower() == "gm"
        lex_scored: list[tuple[float, dict[str, Any]]] = []
        for row in self._rows:
            if corpus_set is not None and row["corpus"] not in corpus_set:
                continue
            if not is_gm and row["visibility"] != "PUBLIC":
                continue
            search_low = str(row["search_text"]).lower()
            sym_low = str(row["symbol_or_section"]).lower()
            crumb_low = str(row["breadcrumb"]).lower()
            src_low = str(row["source_path"]).lower()
            score = 0.0
            if q_low in sym_low or q_low in crumb_low:
                score += 18.0
            elif q_low in search_low:
                score += 10.0
            distinct_hits = 0
            for kw in q_tok_set:
                in_sym = kw in sym_low or kw in crumb_low or kw in src_low
                in_body = kw in search_low
                if in_sym:
                    score += 5.5
                    distinct_hits += 1
                elif in_body:
                    score += 2.2 + min(search_low.count(kw), 5) * 0.4
                    distinct_hits += 1
            if distinct_hits >= 2:
                score += distinct_hits * 2.0
            if score > 0.0:
                lex_scored.append((score, row))

        lex_scored.sort(key=lambda item: (item[0], item[1]["chunk_id"]), reverse=True)
        for rank_idx, (_, row) in enumerate(lex_scored[: max(24, top_k * 6)]):
            cid = str(row["chunk_id"])
            if cid not in fts_ranks:
                fts_ranks[cid] = rank_idx + 1
            candidate_map.setdefault(cid, row)

        # Reciprocal Rank Fusion + arc / exact-heading alignment boost.
        fused: list[tuple[float, dict[str, Any]]] = []
        arc_low = arc_stage.strip().upper()
        for cid, row in candidate_map.items():
            if not is_gm and row.get("visibility") != "PUBLIC":
                continue
            rrf = 0.0
            if cid in vec_ranks:
                rrf += 1.0 / (50.0 + vec_ranks[cid])
            if cid in fts_ranks:
                rrf += 1.15 / (50.0 + fts_ranks[cid])
            if q_low and (q_low in str(row.get("symbol_or_section") or "").lower()):
                rrf += 0.025
            if arc_low.startswith("ARC_1") and row.get("visibility") == "ARC2_SATYANG":
                rrf *= 0.85
            fused.append((rrf, row))

        fused.sort(key=lambda item: (item[0], item[1]["chunk_id"]), reverse=True)
        results: list[dict[str, Any]] = []
        seen_ids: set[str] = set()
        for rrf_score, row in fused[:top_k]:
            item = dict(row)
            item["score"] = round(float(rrf_score), 5)
            results.append(item)
            seen_ids.add(str(row["chunk_id"]))

        if include_linked_rust and len(results) > 0:
            doc_paths = {
                str(r.get("linked_doc_path") or "")
                for r in results
                if r.get("corpus") == "ttrpg_rules" and r.get("linked_doc_path")
            }
            if doc_paths and not any(r.get("corpus") == "rust_engine" for r in results):
                rust_candidates = self.search(
                    q_clean,
                    corpora=["rust_engine"],
                    caller_role=caller_role,
                    arc_stage=arc_stage,
                    top_k=3,
                    include_linked_rust=False,
                )
                picked = None
                for rc in rust_candidates:
                    if str(rc.get("linked_doc_path") or "") in doc_paths:
                        picked = rc
                        break
                if picked is None and rust_candidates:
                    picked = rust_candidates[0]
                if picked is not None and str(picked["chunk_id"]) not in seen_ids:
                    results.append(picked)

        return results


_RAG_INDEX_LOCK = threading.Lock()
_RAG_INDEX_SINGLETON: SatyaHybridRagIndex | None = None


def get_satya_rag_index(*, force_rebuild: bool = False) -> SatyaHybridRagIndex:
    """Return cached `SatyaHybridRagIndex`, building/reloading lazily on first use."""
    global _RAG_INDEX_SINGLETON
    with _RAG_INDEX_LOCK:
        if _RAG_INDEX_SINGLETON is None or force_rebuild:
            _RAG_INDEX_SINGLETON = SatyaHybridRagIndex(force_rebuild=force_rebuild)
        return _RAG_INDEX_SINGLETON


def reset_rag_index_cache() -> None:
    """Reset in-process cached RAG handle (primarily for isolated tests)."""
    global _RAG_INDEX_SINGLETON
    with _RAG_INDEX_LOCK:
        _RAG_INDEX_SINGLETON = None
