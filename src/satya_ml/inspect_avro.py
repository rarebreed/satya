"""CLI utility for inspecting Satya Avro schemas, topic batch watermarks, and PyLance tables."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import fastavro
import lance


def inspect_schemas(base_dir: Path) -> None:
    schemas_dir = base_dir / "schemas"
    if not schemas_dir.exists():
        print("No schemas/ directory found.")
        return

    print("=== SATYA VERSIONED AVRO SCHEMAS ===")
    for avsc_path in sorted(schemas_dir.glob("v*/*.avsc")):
        raw = json.loads(avsc_path.read_text(encoding="utf-8"))
        parsed = fastavro.schema.parse_schema(raw)
        version_dir = avsc_path.parent.name
        fields = [f["name"] for f in raw.get("fields", [])]
        print(
            f"- [{version_dir}] {raw.get('namespace')}.{raw.get('name')} "
            f"({avsc_path.relative_to(base_dir)}) -> {len(fields)} fields: {', '.join(fields)}"
        )
        assert parsed is not None


def inspect_topics(base_dir: Path) -> None:
    topics_dir = base_dir / "data" / "topics"
    lance_dir = base_dir / "data" / "lance"

    print("=== SATYA AVRO TOPIC BATCHES ===")
    if not topics_dir.exists():
        print("No data/topics/ directory found yet. Run the server and flush a batch first.")
    else:
        for batch_file in sorted(topics_dir.glob("*/schema_version=*/partition=*/batch_*.avro")):
            with batch_file.open("rb") as fp:
                reader = fastavro.reader(fp)
                meta = {
                    k: (v.decode("utf-8") if isinstance(v, bytes) else str(v))
                    for k, v in reader.metadata.items()
                    if k.startswith("satya.")
                }
                count = sum(1 for _ in reader)
            print(
                f"- {batch_file.relative_to(base_dir)}: {count} records | "
                f"offsets=[{meta.get('satya.start_offset')}..{meta.get('satya.end_offset')}] | "
                f"schema=v{meta.get('satya.schema_version')} ({meta.get('satya.schema_fingerprint_crc64', '')[:10]}...)"
            )

    print("\n=== SATYA PYLANCE (.lance) AI TABLES ===")
    if not lance_dir.exists():
        print("No data/lance/ directory found yet. Run `pixi run compact-to-lance` to create Lance tables.")
    else:
        for ds_path in sorted(lance_dir.glob("*.lance")):
            ds = lance.dataset(str(ds_path))
            print(f"- {ds_path.name}: {ds.count_rows()} total rows | schema columns: {ds.schema.names}")


def main() -> None:
    parser = argparse.ArgumentParser(description="Inspect Satya Avro schemas, batches, and Lance tables.")
    parser.add_argument("--schemas", action="store_true", help="Validate and list all .avsc schemas")
    parser.add_argument("--topics", action="store_true", help="Inspect .avro topic batches and .lance datasets")
    args = parser.parse_args()

    base_dir = Path.cwd()
    if args.schemas:
        inspect_schemas(base_dir)
    if args.topics:
        inspect_topics(base_dir)
    if not args.schemas and not args.topics:
        inspect_schemas(base_dir)
        print()
        inspect_topics(base_dir)


if __name__ == "__main__":
    main()
