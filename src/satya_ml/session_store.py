"""Shared thread-safe per-session `MutableMapping` stores and reset registries for AI GM and AI Player."""

from __future__ import annotations

from collections.abc import Callable, Iterator, MutableMapping
import threading
from typing import Final

ResetCallback = Callable[[str | None], None]


class SessionResetRegistry:
    """Thread-safe, reentrancy-safe registry of session-scoped state reset hooks."""

    def __init__(self) -> None:
        self._lock = threading.RLock()
        self._callbacks: list[ResetCallback] = []
        self._active_reset_keys: set[str | None] = set()

    def register(self, callback: ResetCallback) -> ResetCallback:
        """Register `callback(session_id)` once and return it."""
        with self._lock:
            if callback not in self._callbacks:
                self._callbacks.append(callback)
        return callback

    def reset(self, session_id: str | None = None) -> None:
        """Invoke all registered reset callbacks once per active key (reentrancy-safe)."""
        with self._lock:
            if session_id in self._active_reset_keys:
                return
            self._active_reset_keys.add(session_id)
            callbacks = list(self._callbacks)
        try:
            for cb in callbacks:
                cb(session_id)
        finally:
            with self._lock:
                self._active_reset_keys.discard(session_id)


GM_SESSION_REGISTRY: Final[SessionResetRegistry] = SessionResetRegistry()
PLAYER_SESSION_REGISTRY: Final[SessionResetRegistry] = SessionResetRegistry()


class _LockedStore[K, T](MutableMapping[K, T]):
    """Thread-safe `MutableMapping` base that auto-registers `reset` on a `SessionResetRegistry`."""

    __slots__ = ("_data", "_lock")

    def __init__(self, *, registry: SessionResetRegistry | None = GM_SESSION_REGISTRY) -> None:
        self._lock = threading.RLock()
        self._data: dict[K, T] = {}
        if registry is not None:
            registry.register(self.reset)

    def __getitem__(self, key: K) -> T:
        with self._lock:
            return self._data[key]

    def __setitem__(self, key: K, value: T) -> None:
        with self._lock:
            self._data[key] = value

    def __delitem__(self, key: K) -> None:
        with self._lock:
            del self._data[key]

    def __iter__(self) -> Iterator[K]:
        with self._lock:
            keys = list(self._data.keys())
        return iter(keys)

    def __len__(self) -> int:
        with self._lock:
            return len(self._data)

    def reset(self, session_id: str | None = None) -> None:
        """Clear all entries (when `session_id is None`) or entries scoped to `session_id`."""
        with self._lock:
            self._data.clear()


class SessionStore[T](_LockedStore[str, T]):
    """Thread-safe per-session mapping keyed by `session_id` with auto-reset registration."""

    def reset(self, session_id: str | None = None) -> None:
        with self._lock:
            if session_id is None:
                self._data.clear()
            else:
                self._data.pop(session_id, None)


class CompositeSessionStore[K: tuple[str, ...], T](_LockedStore[K, T]):
    """Thread-safe composite-key mapping (first tuple item is `session_id`) with auto-reset."""

    def for_session(self, session_id: str) -> list[T]:
        """Return stored entries whose leading `session_id` element matches."""
        with self._lock:
            return [val for key, val in self._data.items() if key[0] == session_id]

    def reset(self, session_id: str | None = None) -> None:
        with self._lock:
            if session_id is None:
                self._data.clear()
                return
            for key in [k for k in self._data if k[0] == session_id]:
                del self._data[key]


__all__ = [
    "CompositeSessionStore",
    "GM_SESSION_REGISTRY",
    "PLAYER_SESSION_REGISTRY",
    "ResetCallback",
    "SessionResetRegistry",
    "SessionStore",
]
