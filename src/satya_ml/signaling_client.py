"""Shared HTTP signaling client, service auth headers, and `config/ai.toml` config loader."""

from __future__ import annotations

from collections.abc import Iterator
from contextlib import contextmanager
from contextvars import ContextVar
import os
from pathlib import Path
import tomllib
from typing import Any, Final

import httpx

from satya_ml.ai_gm.telemetry import current_trace_headers, record_emitted_event_id

OFFLINE_SIGNALING_VALUES: Final[frozenset[str]] = frozenset(
    {"", "0", "false", "none", "off", "offline", "disabled"}
)
_OFFLINE_OVERRIDE_VAR: ContextVar[bool | None] = ContextVar(
    "satya_offline_signaling_override", default=None
)


@contextmanager
def offline_signaling_scope(enabled: bool = True) -> Iterator[None]:
    """Temporarily force `signaling_base_url()` offline (`""`) for the current execution context."""
    tok = _OFFLINE_OVERRIDE_VAR.set(True if enabled else False)
    try:
        yield
    finally:
        _OFFLINE_OVERRIDE_VAR.reset(tok)


def repo_root() -> Path:
    """Return the workspace repository root directory."""
    return Path(__file__).resolve().parent.parent.parent


def load_ai_gm_toml_config(root: Path | None = None) -> dict[str, Any]:
    """Load `[ai_gm]` defaults from `config/ai.toml` if present."""
    ai_toml_path = (root or repo_root()) / "config" / "ai.toml"
    if not ai_toml_path.exists():
        return {}
    try:
        raw = tomllib.loads(ai_toml_path.read_text(encoding="utf-8"))
        ai_gm_cfg = raw.get("ai_gm")
        return dict(ai_gm_cfg) if isinstance(ai_gm_cfg, dict) else {}
    except Exception:
        return {}


def signaling_base_url() -> str:
    """Resolve `SATYA_SIGNALING_URL` (`""` when explicitly disabled/offline)."""
    if _OFFLINE_OVERRIDE_VAR.get() is True:
        return ""
    raw = os.environ.get("SATYA_SIGNALING_URL", "http://127.0.0.1:3000").strip()
    if raw.lower() in OFFLINE_SIGNALING_VALUES:
        return ""
    return raw.rstrip("/")


def is_loopback_url(url: str) -> bool:
    """Return `True` if `url` targets loopback (`127.0.0.1` or `localhost`)."""
    return "127.0.0.1" in url or "localhost" in url


def httpx_timeout(url: str, timeout: float) -> Any:
    """Return an `httpx.Timeout` with fast loopback connect failure detection."""
    if is_loopback_url(url):
        return httpx.Timeout(timeout, connect=min(timeout, 0.25))
    return timeout


def gm_service_token() -> str:
    """Resolve the shared internal GM/Player signaling service bearer token."""
    env_tok = os.environ.get("SATYA_GM_SERVICE_TOKEN")
    if env_tok:
        return env_tok
    cfg_tok = load_ai_gm_toml_config().get("service_token")
    return str(cfg_tok) if cfg_tok else "satya_gm_secret_dev_token"


def service_auth_headers(*, include_trace: bool = True) -> dict[str, str]:
    """Build standard internal service JSON headers with optional W3C trace context."""
    headers: dict[str, str] = {
        "Authorization": f"Bearer {gm_service_token()}",
        "Content-Type": "application/json",
    }
    if include_trace:
        headers.update(current_trace_headers())
    return headers


def _extract_and_record_event_id(res_dict: dict[str, Any]) -> None:
    meta = res_dict.get("metadata")
    if isinstance(meta, dict) and meta.get("event_id"):
        record_emitted_event_id(str(meta["event_id"]))
    elif res_dict.get("message_id"):
        record_emitted_event_id(str(res_dict["message_id"]))


def http_post_json_detailed(
    url: str,
    payload: dict[str, Any],
    timeout: float = 2.5,
    *,
    include_trace: bool = True,
    record_event_id: bool = True,
    client_cls: Any | None = None,
) -> tuple[dict[str, Any] | None, int | None, str | None]:
    """POST JSON to `url`, returning `(parsed_json, status_code, error_body)`."""
    if not url.startswith(("http://", "https://")):
        return None, None, "invalid_url"
    headers = service_auth_headers(include_trace=include_trace)
    eff_client = client_cls or httpx.Client
    try:
        with eff_client(timeout=httpx_timeout(url, timeout)) as client:
            resp = client.post(url, headers=headers, json=payload)
            if resp is None:
                return None, None, None
            if resp.status_code == 200:
                data = dict(resp.json())
                if record_event_id:
                    _extract_and_record_event_id(data)
                return data, 200, None
            return None, int(resp.status_code), (resp.text or f"HTTP {resp.status_code}")[:400]
    except Exception:
        return None, None, None


def http_post_json(
    url: str,
    payload: dict[str, Any],
    timeout: float = 2.5,
    *,
    include_trace: bool = True,
    record_event_id: bool = True,
    client_cls: Any | None = None,
) -> dict[str, Any] | None:
    """POST JSON to `url`, returning parsed JSON on 200 OK or `None` on error."""
    data, _, _ = http_post_json_detailed(
        url,
        payload,
        timeout=timeout,
        include_trace=include_trace,
        record_event_id=record_event_id,
        client_cls=client_cls,
    )
    return data


def http_get_json(
    url: str,
    timeout: float = 2.5,
    *,
    include_trace: bool = True,
    client_cls: Any | None = None,
) -> dict[str, Any] | None:
    """GET JSON from `url`, returning parsed JSON on 200 OK or `None` on failure."""
    if not url.startswith(("http://", "https://")):
        return None
    headers = service_auth_headers(include_trace=include_trace)
    eff_client = client_cls or httpx.Client
    try:
        with eff_client(timeout=httpx_timeout(url, timeout)) as client:
            resp = client.get(url, headers=headers)
            if resp is not None and resp.status_code == 200:
                return dict(resp.json())
    except Exception:
        return None
    return None


def http_delete_json_detailed(
    url: str,
    timeout: float = 2.5,
    *,
    include_trace: bool = True,
    client_cls: Any | None = None,
) -> tuple[dict[str, Any] | None, int | None, str | None]:
    """DELETE JSON at `url`, returning `(parsed_json, status_code, error_body)`."""
    if not url.startswith(("http://", "https://")):
        return None, None, "invalid_url"
    headers = service_auth_headers(include_trace=include_trace)
    eff_client = client_cls or httpx.Client
    try:
        with eff_client(timeout=httpx_timeout(url, timeout)) as client:
            resp = client.delete(url, headers=headers)
            if resp is None:
                return None, None, None
            if resp.status_code == 200:
                return dict(resp.json()), 200, None
            return None, int(resp.status_code), (resp.text or f"HTTP {resp.status_code}")[:400]
    except Exception:
        return None, None, None


def http_delete_json(
    url: str,
    timeout: float = 2.5,
    *,
    include_trace: bool = True,
    client_cls: Any | None = None,
) -> dict[str, Any] | None:
    """DELETE `url`, returning parsed JSON on 200 OK or `None` on failure."""
    data, _, _ = http_delete_json_detailed(
        url,
        timeout=timeout,
        include_trace=include_trace,
        client_cls=client_cls,
    )
    return data


__all__ = [
    "OFFLINE_SIGNALING_VALUES",
    "gm_service_token",
    "http_delete_json",
    "http_delete_json_detailed",
    "http_get_json",
    "http_post_json",
    "http_post_json_detailed",
    "httpx_timeout",
    "is_loopback_url",
    "load_ai_gm_toml_config",
    "offline_signaling_scope",
    "repo_root",
    "service_auth_headers",
    "signaling_base_url",
]
