"""Shared pytest configuration for Satya Python unit and integration tests."""

from __future__ import annotations

import os
import sys
from pathlib import Path

import pytest

_SRC_DIR = str(Path(__file__).resolve().parent.parent / "src")
if _SRC_DIR not in sys.path:
    sys.path.insert(0, _SRC_DIR)


@pytest.fixture(autouse=True)
def _default_offline_signaling(monkeypatch: pytest.MonkeyPatch) -> None:
    """Short-circuit signaling HTTP calls during offline unit tests unless overridden."""
    if "SATYA_SIGNALING_URL" not in os.environ:
        monkeypatch.setenv("SATYA_SIGNALING_URL", "offline")
