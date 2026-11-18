"""CLI entry point (`python -m satya_ml.rag --rebuild`) for building the Hybrid PyLance RAG index."""

from __future__ import annotations

import argparse
import json

from satya_ml.rag.hybrid_index import get_satya_rag_index


def main() -> None:
    parser = argparse.ArgumentParser(description="Build & inspect Satya Hybrid PyLance Knowledge RAG")
    parser.add_argument("--rebuild", action="store_true", help="Force re-extraction and re-indexing")
    parser.add_argument("--clean", action="store_true", help="Prune old MVCC versions on satya-knowledge-rag.lance")
    parser.add_argument(
        "--retain-versions",
        type=int,
        default=2,
        help="Number of recent PyLance MVCC versions to retain when cleaning (default: 2)",
    )
    parser.add_argument("--query", type=str, default="", help="Optional smoke query")
    parser.add_argument("--role", type=str, default="gm", choices=["gm", "player"])
    args = parser.parse_args()

    idx = get_satya_rag_index(force_rebuild=args.rebuild)
    if args.clean or (args.rebuild and args.retain_versions != 2):
        idx.cleanup_versions(retain_versions=args.retain_versions)
    print("[Satya RAG] Index stats:", json.dumps(idx.stats(), indent=2))
    if args.query:
        hits = idx.search(args.query, caller_role=args.role, top_k=4, include_linked_rust=True)
        print(f"[Satya RAG] Top hits for `{args.query}` (role={args.role}):")
        for h in hits:
            print(f" - [{h['corpus']} | {h['visibility']} | score={h['score']}] {h['breadcrumb']} ({h['source_path']})")


if __name__ == "__main__":
    main()
