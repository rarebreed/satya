"""Leaf decorator, metadata registry, and shared session-reset hub for Satya AI GM tools.

Eliminates duplicated ``_FallbackToolWrapper`` blocks, hand-maintained tool-name lists across
``tools.py`` / ``tool_registry.py`` / ``graph.py``, and fragile circular reset imports.
"""

from __future__ import annotations

import importlib
import inspect
from collections.abc import Callable, Iterator, Mapping, MutableMapping
from dataclasses import dataclass
from types import MappingProxyType
from typing import Any, Final, Literal

try:
    from langchain_core.tools import tool
except ImportError:  # pragma: no cover

    class _FallbackToolWrapper:
        def __init__(self, fn: Callable[..., Any], args_schema: type[Any] | None = None) -> None:
            self._fn = fn
            self.func = fn
            self.name = getattr(fn, "__name__", "tool")
            self.__doc__ = getattr(fn, "__doc__", "")
            self.description = str(getattr(fn, "__doc__", "") or "")
            self.args_schema = args_schema
            if args_schema is not None and hasattr(args_schema, "model_fields"):
                self.args = {k: {"title": k} for k in args_schema.model_fields}
            else:
                self.args = {p: {"title": p} for p in inspect.signature(fn).parameters}

        def __call__(self, *args: Any, **kwargs: Any) -> Any:
            return self._fn(*args, **kwargs)

        def invoke(self, input_dict: dict[str, Any]) -> Any:
            if self.args_schema is not None and hasattr(self.args_schema, "model_validate"):
                validated = self.args_schema.model_validate(input_dict)
                kwargs = {k: getattr(validated, k) for k in self.args_schema.model_fields}
                return self._fn(**kwargs)
            return self._fn(**input_dict)

    def tool(fn: Any = None, *, args_schema: type[Any] | None = None, **_kwargs: Any) -> Any:
        if fn is None:
            return lambda f: _FallbackToolWrapper(f, args_schema=args_schema)
        return _FallbackToolWrapper(fn, args_schema=args_schema)


type ToolTier = Literal["TIER_1_SATYA_CORE", "TIER_2_RULESET_ENGINE", "TIER_3_LORE_PROVIDER"]
type ToolTierLabel = Literal["satya-core", "satya-d20-rules", "satya-campaign-lore"]
type DeferredPackId = Literal[
    "combat",
    "tactical_maps",
    "social_and_psyche",
    "world_and_director",
    "tech_and_tradecraft",
    "awakened_campaign",
]
type ToolPackId = Literal["core", "meta"] | DeferredPackId

TIER_LABELS: Final[Mapping[ToolTier, ToolTierLabel]] = MappingProxyType(
    {
        "TIER_1_SATYA_CORE": "satya-core",
        "TIER_2_RULESET_ENGINE": "satya-d20-rules",
        "TIER_3_LORE_PROVIDER": "satya-campaign-lore",
    }
)

_DOMAIN_TOOL_MODULES: Final[tuple[str, ...]] = (
    "satya_ml.ai_gm.tools",
    "satya_ml.ai_gm.maps",
    "satya_ml.ai_gm.maps_tactical",
    "satya_ml.ai_gm.scenario_director",
    "satya_ml.ai_gm.social_and_clocks",
    "satya_ml.ai_gm.npc_psyche_combat",
    "satya_ml.ai_gm.audit_and_first_contact",
    "satya_ml.ai_gm.awakened_systems",
    "satya_ml.ai_gm.tech_and_tradecraft",
)


@dataclass(frozen=True, slots=True)
class ToolSpec:
    """Single-source metadata attached to every `@satya_gm_tool`-decorated LangChain tool."""

    name: str
    tool_obj: Any
    pack: ToolPackId
    tier: ToolTier
    tier_label: ToolTierLabel
    ooc_readonly: bool = False
    session_scoped: bool = True
    campaign_packs: tuple[str, ...] | None = None


_REGISTERED_GM_TOOLS: Final[dict[str, ToolSpec]] = {}
_SESSION_RESET_HOOKS: Final[list[Callable[[str | None], None]]] = []
_ACTIVE_RESET_KEYS: Final[set[str | None]] = set()
_IMPORTING_MODULES: bool = False


def register_session_reset_hook(
    fn: Callable[[str | None], None],
) -> Callable[[str | None], None]:
    """Register a per-module cleanup callback `(session_id | None) -> None` for session resets."""
    if fn not in _SESSION_RESET_HOOKS:
        _SESSION_RESET_HOOKS.append(fn)
    return fn


class SessionStore[T](MutableMapping[str, T]):
    """Per-session mapping keyed by `session_id` that auto-registers with `run_session_reset_hooks`."""

    __slots__ = ("_data",)

    def __init__(self) -> None:
        self._data: dict[str, T] = {}
        register_session_reset_hook(self.reset)

    def __getitem__(self, session_id: str) -> T:
        return self._data[session_id]

    def __setitem__(self, session_id: str, value: T) -> None:
        self._data[session_id] = value

    def __delitem__(self, session_id: str) -> None:
        del self._data[session_id]

    def __iter__(self) -> Iterator[str]:
        return iter(self._data)

    def __len__(self) -> int:
        return len(self._data)

    def reset(self, session_id: str | None = None) -> None:
        """Clear all sessions (when `session_id is None`) or drop a single session."""
        if session_id is None:
            self._data.clear()
            return
        self._data.pop(session_id, None)


class CompositeSessionStore[K: tuple[str, ...], T](MutableMapping[K, T]):
    """Per-session composite-key mapping (first tuple item is `session_id`) with auto-reset."""

    __slots__ = ("_data",)

    def __init__(self) -> None:
        self._data: dict[K, T] = {}
        register_session_reset_hook(self.reset)

    def __getitem__(self, key: K) -> T:
        return self._data[key]

    def __setitem__(self, key: K, value: T) -> None:
        self._data[key] = value

    def __delitem__(self, key: K) -> None:
        del self._data[key]

    def __iter__(self) -> Iterator[K]:
        return iter(self._data)

    def __len__(self) -> int:
        return len(self._data)

    def for_session(self, session_id: str) -> list[T]:
        """Return stored entries whose leading `session_id` element matches."""
        return [val for key, val in self._data.items() if key[0] == session_id]

    def reset(self, session_id: str | None = None) -> None:
        """Clear all composite entries or drop every key belonging to `session_id`."""
        if session_id is None:
            self._data.clear()
            return
        for key in [k for k in self._data if k[0] == session_id]:
            del self._data[key]


def run_session_reset_hooks(session_id: str | None = None) -> None:
    """Invoke every registered GM session-reset hook once (reentrancy-safe)."""
    if session_id in _ACTIVE_RESET_KEYS:
        return
    _ACTIVE_RESET_KEYS.add(session_id)
    try:
        for hook in list(_SESSION_RESET_HOOKS):
            hook(session_id)
    finally:
        _ACTIVE_RESET_KEYS.discard(session_id)


def satya_gm_tool(
    *,
    pack: ToolPackId,
    tier: ToolTier,
    ooc_readonly: bool = False,
    session_scoped: bool = True,
    args_schema: type[Any] | None = None,
    campaign_packs: tuple[str, ...] | None = None,
) -> Callable[[Callable[..., Any]], Any]:
    """Decorate a function as a LangChain `@tool` and register its `ToolSpec` metadata."""
    if tier not in TIER_LABELS:
        raise ValueError(f"Unknown telemetry tier '{tier}'; expected one of {tuple(TIER_LABELS)}")

    def _decorator(fn: Callable[..., Any]) -> Any:
        wrapped = tool(args_schema=args_schema)(fn) if args_schema is not None else tool(fn)
        t_name = str(getattr(wrapped, "name", "") or getattr(fn, "__name__", "")).strip()
        _REGISTERED_GM_TOOLS[t_name] = ToolSpec(
            name=t_name,
            tool_obj=wrapped,
            pack=pack,
            tier=tier,
            tier_label=TIER_LABELS[tier],
            ooc_readonly=ooc_readonly,
            session_scoped=session_scoped,
            campaign_packs=campaign_packs,
        )
        return wrapped

    return _decorator


def ensure_all_gm_tool_modules_imported(*, include_registry: bool = False) -> None:
    """Import all AI GM tool modules idempotently so `_REGISTERED_GM_TOOLS` is complete."""
    global _IMPORTING_MODULES
    if _IMPORTING_MODULES:
        return
    _IMPORTING_MODULES = True
    try:
        for mod_name in _DOMAIN_TOOL_MODULES:
            importlib.import_module(mod_name)
        if include_registry:
            importlib.import_module("satya_ml.ai_gm.tool_registry")
    finally:
        _IMPORTING_MODULES = False


def get_registered_tool_specs(*, include_meta: bool = True) -> dict[str, ToolSpec]:
    """Return all registered `ToolSpec` entries keyed by tool name."""
    ensure_all_gm_tool_modules_imported(include_registry=include_meta)
    if include_meta:
        return dict(_REGISTERED_GM_TOOLS)
    return {k: v for k, v in _REGISTERED_GM_TOOLS.items() if v.pack != "meta"}


def get_builtin_tool_objects() -> list[Any]:
    """Return the 45 built-in AI GM tool objects derived from `@satya_gm_tool` registrations."""
    specs = get_registered_tool_specs(include_meta=False)
    return [spec.tool_obj for spec in specs.values()]


def get_core_tool_names() -> tuple[str, ...]:
    """Return always-bound Core GM tool names derived from `@satya_gm_tool(pack='core')`."""
    specs = get_registered_tool_specs(include_meta=False)
    return tuple(spec.name for spec in specs.values() if spec.pack == "core")


def get_ooc_readonly_tool_names() -> tuple[str, ...]:
    """Return read-only OOC pre-activation tool names derived from `@satya_gm_tool(ooc_readonly=True)`."""
    specs = get_registered_tool_specs(include_meta=False)
    return tuple(spec.name for spec in specs.values() if spec.ooc_readonly)


def get_pack_tool_names(pack_id: ToolPackId) -> tuple[str, ...]:
    """Return tool names registered under deferred domain `pack_id`."""
    specs = get_registered_tool_specs(include_meta=False)
    return tuple(spec.name for spec in specs.values() if spec.pack == pack_id)


def classify_registered_tool_tier(tool_name: str) -> tuple[ToolTier, ToolTierLabel]:
    """Return `(tier_id, metric_label)` for `tool_name` (`TIER_3_LORE_PROVIDER` for external MCP)."""
    ensure_all_gm_tool_modules_imported(include_registry=True)
    spec = _REGISTERED_GM_TOOLS.get(tool_name)
    if spec is not None:
        return (spec.tier, spec.tier_label)
    return ("TIER_3_LORE_PROVIDER", TIER_LABELS["TIER_3_LORE_PROVIDER"])


def is_registered_session_scoped(tool_name: str) -> bool | None:
    """Return declared `session_scoped` flag for registered tools, or `None` if unknown/external."""
    spec = _REGISTERED_GM_TOOLS.get(tool_name)
    return None if spec is None else spec.session_scoped


__all__ = [
    "CompositeSessionStore",
    "DeferredPackId",
    "SessionStore",
    "TIER_LABELS",
    "ToolPackId",
    "ToolSpec",
    "ToolTier",
    "ToolTierLabel",
    "classify_registered_tool_tier",
    "ensure_all_gm_tool_modules_imported",
    "get_builtin_tool_objects",
    "get_core_tool_names",
    "get_ooc_readonly_tool_names",
    "get_pack_tool_names",
    "get_registered_tool_specs",
    "is_registered_session_scoped",
    "register_session_reset_hook",
    "run_session_reset_hooks",
    "satya_gm_tool",
    "tool",
]
