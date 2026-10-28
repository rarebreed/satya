"""Unit tests for the Satya Avro schema inspector and PyLance compaction pipeline."""

from __future__ import annotations

import json
import shutil
import sys
import uuid
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))

import fastavro
import lance

from satya_ml.inspect_avro import inspect_schemas
from satya_ml.lance_ingest import compact_topic_to_lance, read_avro_batch_with_metadata


def test_all_versioned_avro_schemas_parse_with_fastavro() -> None:
    """Verify all .avsc schemas under schemas/v1 and schemas/v2 parse cleanly with fastavro."""
    repo_root = Path(__file__).resolve().parent.parent
    avsc_files = sorted((repo_root / "schemas").glob("v*/*.avsc"))
    assert len(avsc_files) == 7

    for avsc_path in avsc_files:
        raw = json.loads(avsc_path.read_text(encoding="utf-8"))
        parsed = fastavro.schema.parse_schema(raw)
        assert parsed is not None

    inspect_schemas(repo_root)


def test_incremental_avro_batch_to_lance_compaction() -> None:
    """Verify offset-watermarked .avro batches compact into a .lance dataset without duplicates."""
    repo_root = Path(__file__).resolve().parent.parent
    tmp_path = repo_root / "target" / f"py_test_{uuid.uuid4().hex}"
    tmp_path.mkdir(parents=True, exist_ok=True)
    try:
        schema_raw = json.loads(
            (repo_root / "schemas" / "v1" / "session_lifecycle_event.avsc").read_text(
                encoding="utf-8"
            )
        )
        parsed_schema = fastavro.schema.parse_schema(schema_raw)

        topic_dir = tmp_path / "topics" / "session-lifecycle"
        part_dir = topic_dir / "schema_version=v1" / "partition=0"
        part_dir.mkdir(parents=True, exist_ok=True)

        batch_file = part_dir / "batch_0000000000_0000000001.avro"
        records = [
            {
                "metadata": {
                    "event_id": "ev-1",
                    "sequence_num": 1,
                    "schema_subject": "satya.session.lifecycle",
                    "schema_version": 1,
                    "schema_fingerprint_crc64": b"\x01\x02\x03\x04\x05\x06\x07\x08",
                    "session_id": "ep-test",
                    "session_mode": "TTRPG",
                    "client_timestamp_us": 1700000000000000,
                    "ingest_timestamp_us": 1700000000000000,
                    "timezone_offset_min": -240,
                },
                "lifecycle_type": "SESSION_CREATED",
                "actor": {
                    "user_id": "u-gm",
                    "username": "gm_sean",
                    "display_name": "Sean (GM)",
                    "participant_kind": "HUMAN",
                    "role": "GM",
                    "character_id": None,
                    "ai_agent_info": None,
                },
                "target_participant": None,
                "password_protected": True,
                "roster_snapshot": [],
                "reason": "Created session",
            },
            {
                "metadata": {
                    "event_id": "ev-2",
                    "sequence_num": 2,
                    "schema_subject": "satya.session.lifecycle",
                    "schema_version": 1,
                    "schema_fingerprint_crc64": b"\x01\x02\x03\x04\x05\x06\x07\x08",
                    "session_id": "ep-test",
                    "session_mode": "TTRPG",
                    "client_timestamp_us": 1700000001000000,
                    "ingest_timestamp_us": 1700000001000000,
                    "timezone_offset_min": -240,
                },
                "lifecycle_type": "USER_LOGGED_OUT",
                "actor": {
                    "user_id": "u-gm",
                    "username": "gm_sean",
                    "display_name": "Sean (GM)",
                    "participant_kind": "HUMAN",
                    "role": "GM",
                    "character_id": None,
                    "ai_agent_info": None,
                },
                "target_participant": None,
                "password_protected": True,
                "roster_snapshot": [],
                "reason": "User logged out",
            },
        ]

        with batch_file.open("wb") as fp:
            fastavro.writer(
                fp,
                parsed_schema,
                records,
                metadata={
                    "satya.start_offset": "0",
                    "satya.end_offset": "1",
                    "satya.schema_version": "1",
                },
            )

        meta, read_back = read_avro_batch_with_metadata(batch_file)
        assert meta["satya.start_offset"] == "0"
        assert meta["satya.end_offset"] == "1"
        assert len(read_back) == 2

        lance_root = tmp_path / "lance"
        lance_root.mkdir(parents=True, exist_ok=True)
        state_file = lance_root / "_watermarks.json"

        # First compaction ingests 2 records
        ingested_first = compact_topic_to_lance(topic_dir, lance_root, state_file)
        assert ingested_first == 2

        # Second compaction with no new offsets ingests 0 records (deduplicated by watermark)
        ingested_second = compact_topic_to_lance(topic_dir, lance_root, state_file)
        assert ingested_second == 0

        watermarks = json.loads(state_file.read_text(encoding="utf-8"))
        assert watermarks["session-lifecycle"] == 1
    finally:
        shutil.rmtree(tmp_path, ignore_errors=True)
