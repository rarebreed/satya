"""Shared FastAPI server helpers, payload base class, auth verification, LLM resolution, and lock pools."""

from __future__ import annotations

import threading
from typing import Any, cast

from fastapi import HTTPException
from pydantic import BaseModel, Field

from satya_ml.ai_gm.models import SessionLlmConfig
from satya_ml.signaling_client import gm_service_token, load_ai_gm_toml_config


class SessionLockPool:
    """Thread-safe key-scoped `threading.Lock` pool for turn serialization."""

    def __init__(self) -> None:
        self._guard = threading.Lock()
        self._locks: dict[str, threading.Lock] = {}

    def acquire_lock(self, key: str) -> threading.Lock:
        """Return or create the per-key `threading.Lock`."""
        with self._guard:
            lock = self._locks.get(key)
            if lock is None:
                lock = threading.Lock()
                self._locks[key] = lock
            return lock


class BaseTurnEventPayload(BaseModel):
    """Shared request fields for both AI GM and AI Player internal FastAPI turn endpoints."""

    session_id: str
    trigger_event_id: str | None = None
    event_id: str | None = None
    content: str | None = None
    roster: list[dict[str, Any]] = Field(default_factory=list)
    provider: str | None = None
    model_id: str | None = None
    vision_model_id: str | None = None
    temperature: float | None = None
    base_url: str | None = None
    trace_id: str | None = None
    parent_span_id: str | None = None
    lineage_id: str | None = None


def verify_service_auth(
    authorization: str | None,
    *,
    detail: str = "Unauthorized internal bridge token",
) -> None:
    """Validate internal Bearer token against `SATYA_GM_SERVICE_TOKEN` / `config/ai.toml`."""
    expected = f"Bearer {gm_service_token()}"
    if authorization != expected:
        raise HTTPException(status_code=401, detail=detail)


def _pick(*values: Any, default: Any) -> Any:
    """Return the first value that is neither `None` nor empty string `""`."""
    return next((v for v in values if v is not None and v != ""), default)


def resolve_turn_llm_config(
    payload: BaseModel,
    *,
    default_temperature: float,
    x_satya_provider_key: str | None = None,
    provider_config: dict[str, Any] | None = None,
) -> tuple[SessionLlmConfig, dict[str, Any]]:
    """Resolve `SessionLlmConfig` from request payload, `provider_config`, and `config/ai.toml`."""
    p_cfg = provider_config or {}
    file_cfg = load_ai_gm_toml_config()
    raw_reasoning = p_cfg["reasoning"] if "reasoning" in p_cfg else file_cfg.get("reasoning", False)
    llm_cfg = SessionLlmConfig(
        provider=cast(
            Any,
            str(
                _pick(
                    getattr(payload, "provider", None),
                    p_cfg.get("provider"),
                    file_cfg.get("default_provider"),
                    default="ollama",
                )
            ),
        ),
        model_id=str(
            _pick(
                getattr(payload, "model_id", None),
                p_cfg.get("model_id"),
                file_cfg.get("reasoning_model"),
                default="gemma4:12b",
            )
        ),
        vision_model_id=str(
            _pick(
                getattr(payload, "vision_model_id", None),
                p_cfg.get("vision_model_id"),
                file_cfg.get("vision_model"),
                default="gemma4:12b",
            )
        ),
        temperature=float(
            _pick(
                getattr(payload, "temperature", None),
                p_cfg.get("temperature"),
                file_cfg.get("temperature"),
                default=default_temperature,
            )
        ),
        base_url=str(
            _pick(
                getattr(payload, "base_url", None),
                p_cfg.get("base_url"),
                file_cfg.get("ollama_base_url"),
                default="http://127.0.0.1:11434",
            )
        ),
        num_ctx=int(_pick(p_cfg.get("num_ctx"), file_cfg.get("num_ctx"), default=16384)),
        num_predict=int(
            _pick(p_cfg.get("num_predict"), file_cfg.get("num_predict"), default=2048)
        ),
        reasoning=bool(raw_reasoning) if raw_reasoning is not None else False,
        api_key=x_satya_provider_key if isinstance(x_satya_provider_key, str) else None,
    )
    return llm_cfg, file_cfg


__all__ = [
    "BaseTurnEventPayload",
    "SessionLockPool",
    "resolve_turn_llm_config",
    "verify_service_auth",
]
