"""Shared leaf abstractions for 2d20 roll resolution, Pydantic v2 ABC/Protocol request hierarchies, and union input normalization.

Zero imports from other `satya_ml` modules so every AI GM and AI Player module can reuse
`ContestedRollRequest`, `ToolRequest`, `ContextToolRequest`, `Executable`, `ContextualExecutable`,
`roll_2d20_pairs`, and `UnionToolInput` without circular imports.
"""

from __future__ import annotations

import json
from abc import ABC, abstractmethod
from collections.abc import Mapping, Sequence
from types import MappingProxyType
from typing import Any, ClassVar, Final, Literal, Protocol, runtime_checkable

from pydantic import BaseModel, ConfigDict, Field, model_validator
from satya_py.dice import roll_d20_pair, roll_d20_pairs

type CanonicalRollField = Literal[
    "skill_n",
    "control_c",
    "player_pairs",
    "gm_pair",
    "internal_mod",
    "tool_delta_mod",
    "heat",
]

_LEGACY_ROLL_ALIASES: Final[Mapping[str, CanonicalRollField]] = MappingProxyType(
    {
        "skill_rank": "skill_n",
        "shadower_skill_n": "skill_n",
        "investigator_skill_n": "skill_n",
        "requester_skill_n": "skill_n",
        "holder_skill_n": "skill_n",
        "operator_skill_n": "skill_n",
        "human_domain_n": "skill_n",
        "auditor_n": "skill_n",
        "control_tn": "control_c",
        "shadower_control_c": "control_c",
        "investigator_control_c": "control_c",
        "requester_control_c": "control_c",
        "holder_control_c": "control_c",
        "operator_control_c": "control_c",
        "human_control_c": "control_c",
        "auditor_control_c": "control_c",
        "roll_pairs": "player_pairs",
        "shadower_pairs": "player_pairs",
        "investigator_pairs": "player_pairs",
        "operator_pairs": "player_pairs",
        "human_pairs": "player_pairs",
        "auditor_pairs": "player_pairs",
        "target_gm_pair": "gm_pair",
        "shadower_internal_mod": "internal_mod",
        "shadower_tool_delta_mod": "tool_delta_mod",
        "jurisdiction_heat": "heat",
        "cover_heat": "heat",
        "returning_heat": "heat",
    }
)


def parse_json_obj(raw: str) -> dict[str, Any]:
    """Parse a JSON object string into a `dict`, returning `{}` on empty or invalid input."""
    text = str(raw or "").strip()
    if not text:
        return {}
    try:
        parsed = json.loads(text)
    except Exception:
        return {}
    return dict(parsed) if isinstance(parsed, dict) else {}


def parse_json_list(raw: str) -> list[Any] | None:
    """Parse a JSON array string into a `list`, returning `None` on empty or invalid input."""
    text = str(raw or "").strip()
    if not text:
        return None
    try:
        parsed = json.loads(text)
    except Exception:
        return None
    return parsed if isinstance(parsed, list) else None


def roll_2d20_pairs(count: int) -> list[tuple[int, int]]:
    """Roll `max(1, min(10, count))` cryptographically random `2d20` pairs via Rust CSPRNG."""
    n_eff = max(1, min(10, int(count)))
    return roll_d20_pairs(n_eff)


def resolve_d20_pairs(pairs: Sequence[Sequence[int]] | None, count: int) -> list[tuple[int, int]]:
    """Return explicit `2d20` pairs as integer tuples or generate `count` random pairs."""
    if pairs:
        return [(int(pair[0]), int(pair[1])) for pair in pairs]
    return roll_2d20_pairs(count)


def resolve_gm_pair(pair: Sequence[int] | None) -> tuple[int, int]:
    """Return an explicit opposing `2d20` pair or roll one cryptographically random pair via Rust CSPRNG."""
    if pair is not None and len(pair) >= 2:
        return (int(pair[0]), int(pair[1]))
    return roll_d20_pair()


@runtime_checkable
class Executable[R](Protocol):
    """Structural contract for parameterless executable request models."""

    def execute(self) -> R: ...


@runtime_checkable
class ContextualExecutable[Ctx, R](Protocol):
    """Structural contract for context-parameterized executable request models."""

    def execute(self, context: Ctx, /) -> R: ...


class ToolRequest(BaseModel, ABC):
    """Abstract Pydantic v2 request base requiring concrete subclasses to implement `execute()`."""

    @abstractmethod
    def execute(self) -> dict[str, Any]:
        """Evaluate this request against the authoritative Rust rules engine."""


class ContextToolRequest[Ctx, R = dict[str, Any]](BaseModel, ABC):
    """Abstract Pydantic v2 request base requiring context-dependent `execute(context) -> R`."""

    @abstractmethod
    def execute(self, context: Ctx, /) -> R:
        """Evaluate this request using caller-supplied session/world `context`."""


class PassthroughRequest(ToolRequest, ABC):
    """Abstract base for open-schema dict requests backed directly by Rust evaluators."""

    model_config = ConfigDict(extra="allow")

    def extra_payload(self, *exclude: str) -> dict[str, Any]:
        return self.model_dump(exclude={"mode", *exclude})


class ContestedRollRequest(BaseModel):
    """Shared validated `2d20`-pair roll model with legacy alias normalization and RNG fallback."""

    skill_n: int = Field(default=2, ge=1)
    control_c: int = 13
    player_pairs: list[tuple[int, int]] | None = None
    gm_pair: tuple[int, int] | None = None
    internal_mod: int = 0

    @model_validator(mode="before")
    @classmethod
    def _normalize_legacy_roll_fields(cls, data: Any) -> Any:
        if not isinstance(data, Mapping):
            return data
        out = dict(data)
        for legacy_key, canonical_key in _LEGACY_ROLL_ALIASES.items():
            if legacy_key in out and canonical_key in cls.model_fields:
                out.setdefault(canonical_key, out.pop(legacy_key))
        for pairs_key in ("pairs_json", "roll_pairs_json"):
            raw_pairs = out.pop(pairs_key, None)
            if isinstance(raw_pairs, str):
                decoded_pairs = parse_json_list(raw_pairs)
                if decoded_pairs is not None:
                    out.setdefault("player_pairs", decoded_pairs)
        raw_gm = out.pop("gm_pair_json", None)
        if isinstance(raw_gm, str):
            decoded_gm = parse_json_list(raw_gm)
            if decoded_gm is not None and len(decoded_gm) >= 2:
                out.setdefault("gm_pair", (int(decoded_gm[0]), int(decoded_gm[1])))
        return out

    def resolved_pairs(self, default_count: int | None = None) -> list[tuple[int, int]]:
        count = self.skill_n if default_count is None else default_count
        return resolve_d20_pairs(self.player_pairs, count)

    def resolved_gm_pair(self) -> tuple[int, int]:
        return resolve_gm_pair(self.gm_pair)

    def to_rust_pairs_dict(
        self,
        default_count: int | None = None,
        *,
        include_internal_mod: bool = False,
    ) -> dict[str, Any]:
        """Serialize resolved `2d20` pairs (and optional `internal_mod`) for `satya_py` Rust evaluators."""
        payload: dict[str, Any] = {
            "player_pairs": [list(p) for p in self.resolved_pairs(default_count)],
            "gm_pair": list(self.resolved_gm_pair()),
        }
        if include_internal_mod:
            payload["internal_mod"] = self.internal_mod
        return payload


class UnionToolInput(BaseModel):
    """Shared outer wrapper normalizing nested, flat kwargs, and legacy `params_json` inputs."""

    _mode_aliases: ClassVar[Mapping[str, str]] = MappingProxyType({})
    _default_mode: ClassVar[str] = ""
    _mode_case: ClassVar[Literal["upper", "lower"]] = "upper"
    _outer_keys: ClassVar[tuple[str, ...]] = ("session_id",)

    session_id: str = "default"

    @model_validator(mode="before")
    @classmethod
    def _normalize_input(cls, data: Any) -> Any:
        if not isinstance(data, Mapping):
            return data
        raw = dict(data)
        if isinstance(raw.get("request"), BaseModel):
            return raw

        outer_keys = cls._outer_keys
        outer: dict[str, Any] = {k: raw[k] for k in outer_keys if k in raw}
        if isinstance(raw.get("request"), Mapping):
            req = dict(raw["request"])
        else:
            req = {}
            legacy_params = raw.get("params_json")
            if isinstance(legacy_params, str):
                req.update(parse_json_obj(legacy_params))
            elif isinstance(legacy_params, Mapping):
                req.update(dict(legacy_params))
            for k, v in raw.items():
                if k not in outer_keys and k != "params_json":
                    req[k] = v
            if "mode" in raw:
                req["mode"] = raw["mode"]

        mode_str = str(req.get("mode") or cls._default_mode).strip().replace("-", "_").replace(" ", "_")
        norm_mode = mode_str.lower() if cls._mode_case == "lower" else mode_str.upper()
        req["mode"] = cls._mode_aliases.get(norm_mode, norm_mode)
        outer["request"] = req
        return outer


__all__ = [
    "CanonicalRollField",
    "ContestedRollRequest",
    "ContextToolRequest",
    "ContextualExecutable",
    "Executable",
    "PassthroughRequest",
    "ToolRequest",
    "UnionToolInput",
    "parse_json_list",
    "parse_json_obj",
    "resolve_d20_pairs",
    "resolve_gm_pair",
    "roll_2d20_pairs",
]
