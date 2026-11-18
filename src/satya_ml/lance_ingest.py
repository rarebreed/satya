"""Incremental Avro-to-PyLance (Lance format) batch ingestor for Satya topics and multimodal assets.

Scans `data/topics/<topic>/schema_version=v*/partition=*/batch_*.avro` (including `character-sheets`),
inspects the embedded `satya.start_offset` and `satya.end_offset` Avro header metadata, converts
nested Avro records (`Participant`, `EventMetadata`, `CharacterSheetRecord`) into PyArrow tables,
and appends only new offset ranges into `data/lance/<topic>.lance`.

Also compacts multimodal character assets (`data/characters/assets/<character_id>/<asset_id>_<filename>`)
into `data/lance/character-assets.lance` with binary blob columns (`blob_bytes`) for vision, PDF/RAG,
and 3D `.glb` ML workflows, and provides fragment compaction + MVCC version cleanup across all
`data/lance/*.lance` tables.
"""

from __future__ import annotations

import argparse
from datetime import timedelta
import hashlib
import json
from pathlib import Path
from typing import Any, cast

import fastavro
import lance
import pyarrow as pa

DEFAULT_RETAIN_VERSIONS = 2
OVERWRITE_ONLY_TABLES = frozenset({"satya-knowledge-rag.lance", "character-assets.lance"})


def _empty_cleanup_stats(*, skipped: int = 0) -> dict[str, int]:
    stats = {
        "old_versions_removed": 0,
        "bytes_removed": 0,
        "data_files_removed": 0,
        "index_files_removed": 0,
        "transaction_files_removed": 0,
        "deletion_files_removed": 0,
    }
    if skipped:
        stats["skipped"] = skipped
    return stats


def prune_lance_versions(
    ds: Any,
    *,
    retain_versions: int = DEFAULT_RETAIN_VERSIONS,
) -> dict[str, int]:
    """Prune superseded PyLance MVCC versions while retaining `retain_versions` snapshots.

    Uses `timedelta(microseconds=1)` rather than `timedelta(0)` because PyLance's
    `cleanup_old_versions` treats a falsy `timedelta(0)` as `None` (defaulting to 14 days).
    """
    try:
        stats = ds.cleanup_old_versions(
            older_than=timedelta(microseconds=1),
            retain_versions=max(1, retain_versions),
            delete_unverified=False,
            error_if_tagged_old_versions=False,
        )
        return {
            "old_versions_removed": int(stats.old_versions),
            "bytes_removed": int(stats.bytes_removed),
            "data_files_removed": int(stats.data_files_removed),
            "index_files_removed": int(stats.index_files_removed),
            "transaction_files_removed": int(stats.transaction_files_removed),
            "deletion_files_removed": int(stats.deletion_files_removed),
        }
    except (OSError, RuntimeError, ValueError):
        return _empty_cleanup_stats(skipped=1)


def optimize_lance_dataset(
    lance_uri: Path,
    *,
    compact_fragments: bool = True,
    retain_versions: int = DEFAULT_RETAIN_VERSIONS,
) -> dict[str, int]:
    """Compact fragmented topic tables and prune old MVCC versions on a `.lance` dataset."""
    result: dict[str, int] = {
        "fragments_removed": 0,
        "fragments_added": 0,
        "files_removed": 0,
        "files_added": 0,
        **_empty_cleanup_stats(),
        "active_fragments": 0,
        "active_versions": 0,
    }
    if not lance_uri.exists():
        return result

    ds = lance.dataset(str(lance_uri))
    if compact_fragments and lance_uri.name not in OVERWRITE_ONLY_TABLES and len(ds.get_fragments()) > 1:
        try:
            metrics = ds.optimize.compact_files()
            result["fragments_removed"] = metrics.fragments_removed
            result["fragments_added"] = metrics.fragments_added
            result["files_removed"] = metrics.files_removed
            result["files_added"] = metrics.files_added
            # Compaction.execute commits a new version on disk without rebinding the caller handle.
            ds = lance.dataset(str(lance_uri))
        except (OSError, RuntimeError):
            pass

    cleanup = prune_lance_versions(ds, retain_versions=retain_versions)
    result.update(cleanup)
    ds = lance.dataset(str(lance_uri))
    result["active_fragments"] = len(ds.get_fragments())
    result["active_versions"] = len(ds.versions())
    return result


def clean_all_lance_datasets(
    lance_root: Path,
    *,
    retain_versions: int = DEFAULT_RETAIN_VERSIONS,
) -> dict[str, dict[str, int]]:
    """Compact fragments and prune old MVCC versions across all `.lance` tables in `lance_root`."""
    if not lance_root.exists():
        return {}
    summary: dict[str, dict[str, int]] = {}
    for ds_path in sorted(lance_root.glob("*.lance")):
        if not ds_path.is_dir():
            continue
        summary[ds_path.name] = optimize_lance_dataset(
            ds_path,
            compact_fragments=(ds_path.name not in OVERWRITE_ONLY_TABLES),
            retain_versions=retain_versions,
        )
    return summary


def read_avro_batch_with_metadata(avro_path: Path) -> tuple[dict[str, str], list[dict[str, Any]]]:
    with avro_path.open("rb") as fp:
        reader = fastavro.reader(fp)
        meta: dict[str, str] = {
            str(k): (v.decode("utf-8") if isinstance(v, bytes) else str(v))
            for k, v in reader.metadata.items()
            if str(k).startswith("satya.")
        }
        records = cast(list[dict[str, Any]], list(reader))
    return meta, records


def compact_topic_to_lance(
    topic_dir: Path,
    lance_Root: Path,
    state_file: Path,
    *,
    optimize: bool = True,
    retain_versions: int = DEFAULT_RETAIN_VERSIONS,
) -> int:
    topic_name = topic_dir.name
    avro_files = sorted(topic_dir.glob("schema_version=*/partition=*/batch_*.avro"))
    if not avro_files:
        return 0

    watermarks: dict[str, int] = {}
    if state_file.exists():
        watermarks = json.loads(state_file.read_text(encoding="utf-8"))

    high_water = watermarks.get(topic_name, -1)
    new_records: list[dict[str, Any]] = []
    max_offset_seen = high_water

    for avro_file in avro_files:
        meta, records = read_avro_batch_with_metadata(avro_file)
        end_offset = int(meta.get("satya.end_offset", -1))
        if end_offset <= high_water:
            continue
        for rec in records:
            new_records.append(rec)
        if end_offset > max_offset_seen:
            max_offset_seen = end_offset

    if not new_records:
        return 0

    arrow_table = pa.Table.from_pylist(new_records)
    lance_uri = lance_Root / f"{topic_name}.lance"
    mode = "append" if lance_uri.exists() else "create"
    wrote_to_disk = False
    try:
        lance.write_dataset(arrow_table, str(lance_uri), mode=mode)
        wrote_to_disk = True
    except OSError:
        # Fallback for environments where Windows hardlinks (CreateHardLinkW) are restricted
        lance.write_dataset(arrow_table, f"memory://{topic_name}.lance", mode="overwrite")

    if wrote_to_disk and optimize and mode == "append":
        optimize_lance_dataset(
            lance_uri,
            compact_fragments=True,
            retain_versions=retain_versions,
        )

    watermarks[topic_name] = max_offset_seen
    state_file.parent.mkdir(parents=True, exist_ok=True)
    state_file.write_text(json.dumps(watermarks, indent=2), encoding="utf-8")
    return len(new_records)


def compact_character_assets_to_lance(
    assets_root: Path,
    lance_root: Path,
    *,
    retain_versions: int = DEFAULT_RETAIN_VERSIONS,
) -> int:
    if not assets_root.exists():
        return 0
    rows: list[dict[str, Any]] = []
    for char_dir in sorted(assets_root.iterdir()):
        if not char_dir.is_dir():
            continue
        character_id = char_dir.name
        for asset_file in sorted(char_dir.iterdir()):
            if not asset_file.is_file():
                continue
            blob = asset_file.read_bytes()
            sha256_hex = hashlib.sha256(blob).hexdigest()
            asset_id, _, filename = asset_file.name.partition("_")
            rows.append(
                {
                    "asset_id": asset_id,
                    "character_id": character_id,
                    "filename": filename or asset_file.name,
                    "size_bytes": len(blob),
                    "sha256_hex": sha256_hex,
                    "blob_bytes": blob,
                }
            )
    if not rows:
        return 0
    arrow_table = pa.Table.from_pylist(rows)
    lance_uri = lance_root / "character-assets.lance"
    try:
        ds = lance.write_dataset(arrow_table, str(lance_uri), mode="overwrite")
        prune_lance_versions(ds, retain_versions=retain_versions)
    except OSError:
        lance.write_dataset(arrow_table, "memory://character-assets.lance", mode="overwrite")
    return len(rows)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Compact Avro batches into PyLance datasets and prune old MVCC versions."
    )
    parser.add_argument(
        "--clean-only",
        action="store_true",
        help="Compact small fragments and prune old MVCC versions across data/lance/*.lance without ingesting Avro or rebuilding RAG",
    )
    parser.add_argument(
        "--retain-versions",
        type=int,
        default=DEFAULT_RETAIN_VERSIONS,
        help="Number of recent PyLance MVCC versions to retain during cleanup (default: 2)",
    )
    args = parser.parse_args()

    base_dir = Path.cwd()
    topics_root = base_dir / "data" / "topics"
    assets_root = base_dir / "data" / "characters" / "assets"
    lance_root = base_dir / "data" / "lance"
    state_file = lance_root / "_watermarks.json"

    if args.clean_only:
        summary = clean_all_lance_datasets(lance_root, retain_versions=args.retain_versions)
        if not summary:
            print(f"No PyLance datasets found in {lance_root}")
            return
        for table_name, st in summary.items():
            print(
                f"[PyLance] Cleaned {table_name}: "
                f"versions_removed={st['old_versions_removed']} | "
                f"fragments_compacted={st['fragments_removed']}->{st['fragments_added']} | "
                f"bytes_reclaimed={st['bytes_removed']} | "
                f"active_fragments={st['active_fragments']} | "
                f"active_versions={st['active_versions']}"
            )
        return

    lance_root.mkdir(parents=True, exist_ok=True)
    total_ingested = 0
    if not topics_root.exists():
        print(f"No topic batches directory found at {topics_root}")
    else:
        for topic_dir in sorted(topics_root.iterdir()):
            if topic_dir.is_dir():
                count = compact_topic_to_lance(
                    topic_dir,
                    lance_root,
                    state_file,
                    retain_versions=args.retain_versions,
                )
                if count > 0:
                    print(f"[PyLance] Appended {count} events into {lance_root / (topic_dir.name + '.lance')}")
                    total_ingested += count

    asset_count = compact_character_assets_to_lance(
        assets_root,
        lance_root,
        retain_versions=args.retain_versions,
    )
    if asset_count > 0:
        print(f"[PyLance] Indexed {asset_count} multimodal character assets into {lance_root / 'character-assets.lance'}")

    print(f"[PyLance] Compaction complete. Total new events ingested: {total_ingested}")
    try:
        from satya_ml.rag.hybrid_index import get_satya_rag_index

        rag_idx = get_satya_rag_index(force_rebuild=True)
        print(f"[PyLance] Knowledge RAG refreshed: {rag_idx.stats().get('total_chunks', 0)} chunks.")
    except Exception as exc:
        print(f"[PyLance] Knowledge RAG refresh skipped: {exc}")

    cleanup_summary = clean_all_lance_datasets(lance_root, retain_versions=args.retain_versions)
    for table_name, st in cleanup_summary.items():
        if st["old_versions_removed"] > 0 or st["fragments_removed"] > 0:
            print(
                f"[PyLance] Post-ingest cleanup {table_name}: "
                f"versions_removed={st['old_versions_removed']} | "
                f"fragments_compacted={st['fragments_removed']}->{st['fragments_added']} | "
                f"bytes_reclaimed={st['bytes_removed']}"
            )


if __name__ == "__main__":
    main()
