"""Rust-driven `StrEnum` types synthesized dynamically from `satya_py._native` catalogs."""

from __future__ import annotations

from collections.abc import Mapping
from enum import StrEnum
import re
from types import MappingProxyType
from typing import Any, Final

from satya_py import _native


def _member_key(code: str) -> str:
    raw = (
        code
        if ("_" in code or code == code.upper() or code == code.lower())
        else re.sub(r"(?<=[a-z0-9])([A-Z])", r"_\1", code)
    )
    key = raw.replace("-", "_").upper()
    return f"_{key}" if key[:1].isdigit() else key


def _make_enum(cls_name: str, codes: list[str], doc: str) -> type[StrEnum]:
    enum_factory: Any = StrEnum
    cls: type[StrEnum] = enum_factory(cls_name, {_member_key(c): c for c in codes})
    cls.__module__ = __name__
    cls.__doc__ = doc
    return cls


SchemaDtoName: type[StrEnum] = _make_enum(
    "SchemaDtoName",
    _native.list_schema_dto_names(),
    "Canonical `satya-schema` DTO names accepted by `validate_schema_dto`.",
)
WireEnumName: type[StrEnum] = _make_enum(
    "WireEnumName",
    _native.list_wire_enum_names(),
    "Canonical Rust wire/coded enum names accepted by `list_wire_enum_codes`.",
)

_REGISTRY_MAP: dict[str, type[StrEnum]] = {}
for _enum_name in WireEnumName:
    _cls = _make_enum(
        _enum_name.value,
        _native.list_wire_enum_codes(_enum_name.value),
        f"Typed `StrEnum` mirrored from Rust `{_enum_name.value}`.",
    )
    globals()[_enum_name.value] = _cls
    _REGISTRY_MAP[_enum_name.value] = _cls

RUST_ENUM_PARITY_MAP: Final[Mapping[str, type[StrEnum]]] = MappingProxyType(_REGISTRY_MAP)

CombatStance: type[StrEnum] = _REGISTRY_MAP["CombatStance"]

_COMBAT_STANCE_ALIASES: Final[Mapping[str, StrEnum]] = MappingProxyType(
    {
        alias: CombatStance(canonical)
        for alias, canonical in (
            ("POWER", "POWER"),
            ("AGGRESSIVE", "POWER"),
            ("MANEUVER", "MANEUVER"),
            ("STABILITY", "STABILITY"),
            ("NEUTRAL", "STABILITY"),
            ("DEFENSIVE", "STABILITY"),
            ("RANGED_AIMED", "RANGED_AIMED"),
            ("AIMED", "RANGED_AIMED"),
            ("RANGED_SNAP", "RANGED_SNAP"),
            ("SNAP", "RANGED_SNAP"),
        )
    }
)


def normalize_combat_stance(
    raw: str | StrEnum | None,
    default: StrEnum = CombatStance("STABILITY"),
) -> StrEnum:
    """Normalize canonical or legacy stance strings into canonical `CombatStance`."""
    if isinstance(raw, CombatStance):
        return raw
    if raw is None:
        return default
    key = str(raw).strip().upper().replace("-", "_").replace(" ", "_")
    if not key:
        return default
    resolved = _COMBAT_STANCE_ALIASES.get(key)
    if resolved is not None:
        return resolved
    raise ValueError(f"Unknown combat stance '{raw}'; expected one of {[s.value for s in CombatStance]}")


__all__ = sorted(
    [*_REGISTRY_MAP.keys(), "RUST_ENUM_PARITY_MAP", "SchemaDtoName", "WireEnumName", "normalize_combat_stance"]
)
