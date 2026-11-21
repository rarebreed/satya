"""Extension binary locator and lazy sync helper for ``satya_py._native``."""

from __future__ import annotations

import shutil
from pathlib import Path
from types import ModuleType
from typing import Final

_BUILD_PROFILES: Final[tuple[str, ...]] = ("release", "debug")


def _sync_extension_if_newer(pkg_dir: Path, repo_root: Path) -> None:
    pyd_path = pkg_dir / "_native.pyd"
    candidates = [repo_root / "target" / profile / "satya_py.dll" for profile in _BUILD_PROFILES]
    existing = [p for p in candidates if p.exists()]
    if not existing:
        return
    newest = max(existing, key=lambda p: p.stat().st_mtime)
    if not pyd_path.exists() or newest.stat().st_mtime > pyd_path.stat().st_mtime:
        try:
            shutil.copy2(newest, pyd_path)
        except OSError:
            # Another live server may hold an exclusive lock on `_native.pyd` under Windows.
            pass


def ensure_native_loaded() -> ModuleType:
    """Ensure ``satya_py._native`` is synchronized from ``target/`` and return the module."""
    pkg_dir = Path(__file__).resolve().parent
    repo_root = pkg_dir.parent.parent
    _sync_extension_if_newer(pkg_dir, repo_root)
    from satya_py import _native

    return _native
