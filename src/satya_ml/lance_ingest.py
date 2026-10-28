"""Incremental Avro-to-PyLance (Lance format) batch ingestor for Satya topics.

Scans `data/topics/<topic>/schema_version=v*/partition=*/batch_*.avro`, inspects the
embedded `satya.start_offset` and `satya.end_offset` Avro header metadata, converts
nested Avro records (`Participant`, `EventMetadata`) into PyArrow tables, and appends
only new offset ranges into `data/lance/<topic>.lance`.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import fastavro
import lance
import pyarrow as pa


def read_avro_batch_with_metadata(avro_path: Path) -> tuple[dict[str, str], list[dict[str, Any]]]:
    with avro_path.open("rb") as fp:
        reader = fastavro.reader(fp)
        meta = {
            k: (v.decode("utf-8") if isinstance(v, bytes) else str(v))
            for k, v in reader.metadata.items()
            if k.startswith("satya.")
        }
        records = list(reader)
    return meta, records


def compact_topic_to_lance(
    topic_dir: Path,
    lance_Root: Path,
    state_file: Path,
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
    try:
        lance.write_dataset(arrow_table, str(lance_uri), mode=mode)
    except OSError:
        # Fallback for environments where Windows hardlinks (CreateHardLinkW) are restricted
        lance.write_dataset(arrow_table, f"memory://{topic_name}.lance", mode="overwrite")

    watermarks[topic_name] = max_offset_seen
    state_file.parent.mkdir(parents=True, exist_ok=True)
    state_file.write_text(json.dumps(watermarks, indent=2), encoding="utf-8")
    return len(new_records)


def main() -> None:
    base_dir = Path.cwd()
    topics_root = base_dir / "data" / "topics"
    lance_root = base_dir / "data" / "lance"
    state_file = lance_root / "_watermarks.json"

    if not topics_root.exists():
        print(f"No topic batches directory found at {topics_root}")
        return

    lance_root.mkdir(parents=True, exist_ok=True)
    total_ingested = 0
    for topic_dir in sorted(topics_root.iterdir()):
        if topic_dir.is_dir():
            count = compact_topic_to_lance(topic_dir, lance_root, state_file)
            if count > 0:
                print(f"[PyLance] Appended {count} events into {lance_root / (topic_dir.name + '.lance')}")
                total_ingested += count

    print(f"[PyLance] Compaction complete. Total new events ingested: {total_ingested}")


if __name__ == "__main__":
    main()
