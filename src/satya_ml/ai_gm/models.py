"""Multi-provider LLM factory and ephemeral BYOK configuration for the Satya AI GM."""

from __future__ import annotations

import os
import ssl
from dataclasses import dataclass, field
from typing import Any, Literal

ProviderName = Literal["ollama", "google_genai", "openai", "anthropic"]

_ORIG_CREATE_DEFAULT_CONTEXT = ssl.create_default_context


def _create_default_context_with_system_fallback(
    purpose: ssl.Purpose = ssl.Purpose.SERVER_AUTH,
    *,
    cafile: str | bytes | os.PathLike[Any] | None = None,
    capath: str | bytes | os.PathLike[Any] | None = None,
    cadata: str | bytes | None = None,
) -> ssl.SSLContext:
    """Create an SSLContext, falling back to OS root certs if `cafile`/`capath` raises `PermissionError`."""
    try:
        return _ORIG_CREATE_DEFAULT_CONTEXT(
            purpose=purpose,
            cafile=cafile,
            capath=capath,
            cadata=cadata,
        )
    except PermissionError:
        if cafile is not None or capath is not None:
            return _ORIG_CREATE_DEFAULT_CONTEXT(
                purpose=purpose,
                cafile=None,
                capath=None,
                cadata=cadata,
            )
        raise


if getattr(ssl.create_default_context, "__name__", "") != "_create_default_context_with_system_fallback":
    ssl.create_default_context = _create_default_context_with_system_fallback  # type: ignore[assignment]


def redact_api_key(api_key: str | None) -> str:
    """Return a masked representation of an API key that is safe for logs/diagnostics."""
    if not api_key:
        return "<none>"
    return "[REDACTED_BYOK_KEY]"


@dataclass(frozen=True)
class SessionLlmConfig:
    """Per-session LLM provider configuration for the AI GM."""

    provider: ProviderName = "ollama"
    model_id: str = "gemma4:12b"
    vision_model_id: str | None = "gemma4:12b"
    temperature: float = 0.35
    base_url: str | None = "http://127.0.0.1:11434"
    num_ctx: int = 16384
    num_predict: int = 2048
    reasoning: bool | None = False
    # Ephemeral BYOK key injected per-turn from satya-signaling memory; excluded from repr().
    api_key: str | None = field(default=None, repr=False)

    def target_model_id(self, *, for_vision: bool = False) -> str:
        """Resolve the effective model identifier for text/tool turns vs. multimodal vision turns."""
        if for_vision and self.vision_model_id:
            return self.vision_model_id
        return self.model_id


_LOOPBACK_HOSTS = frozenset({"127.0.0.1", "localhost", "::1"})
_OFFICIAL_PROVIDER_HOSTS: dict[str, frozenset[str]] = {
    "openai": frozenset({"api.openai.com"}),
    "anthropic": frozenset({"api.anthropic.com"}),
    "google_genai": frozenset({"generativelanguage.googleapis.com"}),
    "ollama": _LOOPBACK_HOSTS,
}


def _origin_of(raw_url: str) -> str:
    from urllib.parse import urlparse

    parsed = urlparse((raw_url or "").strip())
    scheme = (parsed.scheme or "").lower()
    netloc = (parsed.netloc or "").lower()
    return f"{scheme}://{netloc}" if scheme and netloc else ""


def _is_custom_non_loopback_base_url(provider: str, base_url: str | None) -> bool:
    raw = str(base_url or "").strip()
    if not raw:
        return False
    from urllib.parse import urlparse

    host = (urlparse(raw).hostname or "").strip().lower()
    if not host:
        return True
    if host in _LOOPBACK_HOSTS:
        return False
    return host not in _OFFICIAL_PROVIDER_HOSTS.get(provider, frozenset())


def _validate_base_url_safety(
    provider: str,
    base_url: str | None,
    *,
    api_key: str | None = None,
) -> tuple[bool, str | None]:
    """Validate `base_url` against SSRF/metadata/untrusted hosts and classify whether it is custom.

    Returns `(is_custom_endpoint, validated_base_url)`.
    """
    raw = str(base_url or "").strip()
    if not raw:
        return False, None

    import ipaddress
    from urllib.parse import urlparse

    from satya_ml.signaling_client import load_ai_gm_toml_config

    parsed = urlparse(raw)
    scheme = (parsed.scheme or "").lower()
    if scheme not in {"http", "https"} or parsed.username or parsed.password:
        raise ValueError("Untrusted provider base_url: scheme must be http/https without embedded credentials")

    host = (parsed.hostname or "").strip().lower()
    if not host:
        raise ValueError("Untrusted provider base_url: missing hostname")

    try:
        ip_obj = ipaddress.ip_address(host)
    except ValueError:
        ip_obj = None

    if ip_obj is not None and (
        ip_obj.is_unspecified
        or ip_obj.is_link_local
        or ip_obj.is_multicast
        or ip_obj.is_reserved
    ):
        raise ValueError(f"Untrusted provider base_url host '{host}': link-local/metadata/unspecified IPs are forbidden")

    file_cfg = load_ai_gm_toml_config()
    raw_allowlist = file_cfg.get("allowed_provider_base_urls") or []
    allowed_origins = {
        _origin_of(str(u))
        for u in raw_allowlist
        if isinstance(u, str) and u.strip()
    }
    req_origin = _origin_of(raw)

    if provider == "ollama":
        default_ollama_origin = _origin_of(
            str(file_cfg.get("ollama_base_url") or os.environ.get("OLLAMA_HOST") or "http://127.0.0.1:11434")
        )
        if host not in _LOOPBACK_HOSTS and req_origin != default_ollama_origin and req_origin not in allowed_origins:
            raise ValueError(
                f"Untrusted Ollama base_url '{req_origin}': non-loopback Ollama endpoints must be allowlisted in allowed_provider_base_urls"
            )
        return False, raw

    official_hosts = _OFFICIAL_PROVIDER_HOSTS.get(provider, frozenset())
    if scheme == "https" and host in official_hosts:
        return False, raw

    # Non-official endpoints for cloud providers (INCLUDING loopback) are ALWAYS custom so server
    # environment API keys (`OPENAI_API_KEY`, etc.) are never forwarded to untrusted destinations.
    if not (api_key and api_key.strip()):
        raise ValueError("Explicit BYOK api_key is required when using a custom provider base_url")
    if host not in _LOOPBACK_HOSTS and req_origin not in allowed_origins:
        raise ValueError(
            f"Untrusted provider base_url '{req_origin}' for provider '{provider}'; add to allowed_provider_base_urls to permit"
        )
    return True, raw


def build_gm_chat_model(cfg: SessionLlmConfig, *, for_vision: bool = False) -> Any:
    """Instantiate a LangChain BaseChatModel supporting tool-calling and multimodal inputs."""
    target_model = cfg.target_model_id(for_vision=for_vision)
    custom_base, validated_base_url = _validate_base_url_safety(
        cfg.provider,
        cfg.base_url,
        api_key=cfg.api_key,
    )

    if cfg.provider == "ollama":
        from langchain_ollama import ChatOllama

        return ChatOllama(
            model=target_model,
            base_url=validated_base_url or os.environ.get("OLLAMA_HOST", "http://127.0.0.1:11434"),
            temperature=cfg.temperature,
            num_ctx=cfg.num_ctx,
            num_predict=cfg.num_predict,
            reasoning=cfg.reasoning,
        )

    if custom_base and not (cfg.api_key and cfg.api_key.strip()):
        raise ValueError("Explicit BYOK api_key is required when using a custom provider base_url")

    if cfg.provider == "google_genai":
        from langchain_google_genai import ChatGoogleGenerativeAI

        api_key = cfg.api_key if custom_base else (
            cfg.api_key
            or os.environ.get("GOOGLE_API_KEY")
            or os.environ.get("GEMINI_API_KEY")
        )
        if not api_key:
            raise ValueError(
                "Missing API key for provider 'google_genai' (set GOOGLE_API_KEY or GEMINI_API_KEY, or pass a session BYOK key)"
            )
        return ChatGoogleGenerativeAI(
            model=target_model,
            google_api_key=api_key,
            temperature=cfg.temperature,
        )

    if cfg.provider == "openai":
        from langchain_openai import ChatOpenAI

        api_key = cfg.api_key if custom_base else (
            cfg.api_key or os.environ.get("OPENAI_API_KEY")
        )
        if not api_key:
            raise ValueError("Missing API key for provider 'openai'")
        kwargs: dict[str, Any] = {
            "model": target_model,
            "api_key": api_key,
            "temperature": cfg.temperature,
        }
        if validated_base_url:
            kwargs["base_url"] = validated_base_url
        return ChatOpenAI(**kwargs)

    if cfg.provider == "anthropic":
        from langchain_anthropic import ChatAnthropic

        api_key = cfg.api_key if custom_base else (
            cfg.api_key or os.environ.get("ANTHROPIC_API_KEY")
        )
        if not api_key:
            raise ValueError("Missing API key for provider 'anthropic'")
        return ChatAnthropic(
            model_name=target_model,
            anthropic_api_key=api_key,
            temperature=cfg.temperature,
        )

    raise ValueError(f"Unsupported LLM provider: {cfg.provider}")
