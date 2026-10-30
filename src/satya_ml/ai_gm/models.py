"""Multi-provider LLM factory and ephemeral BYOK configuration for the Satya AI GM."""

from __future__ import annotations

import os
from dataclasses import dataclass, field
from typing import Any, Literal

ProviderName = Literal["ollama", "google_genai", "openai", "anthropic"]


def redact_api_key(api_key: str | None) -> str:
    """Return a masked representation of an API key that is safe for logs/diagnostics."""
    if not api_key:
        return "<none>"
    return "[REDACTED_BYOK_KEY]"


@dataclass(frozen=True)
class SessionLlmConfig:
    """Per-session LLM provider configuration for the AI GM."""

    provider: ProviderName = "ollama"
    model_id: str = "qwen3:8b"
    vision_model_id: str | None = "qwen2.5vl:7b"
    temperature: float = 0.35
    base_url: str | None = "http://127.0.0.1:11434"
    # Ephemeral BYOK key injected per-turn from satya-signaling memory; excluded from repr().
    api_key: str | None = field(default=None, repr=False)

    def target_model_id(self, *, for_vision: bool = False) -> str:
        """Resolve the effective model identifier for text/tool turns vs. multimodal vision turns."""
        if for_vision and self.vision_model_id:
            return self.vision_model_id
        return self.model_id


def build_gm_chat_model(cfg: SessionLlmConfig, *, for_vision: bool = False) -> Any:
    """Instantiate a LangChain BaseChatModel supporting tool-calling and multimodal inputs."""
    target_model = cfg.target_model_id(for_vision=for_vision)

    if cfg.provider == "ollama":
        from langchain_ollama import ChatOllama

        return ChatOllama(
            model=target_model,
            base_url=cfg.base_url or os.environ.get("OLLAMA_HOST", "http://127.0.0.1:11434"),
            temperature=cfg.temperature,
        )

    if cfg.provider == "google_genai":
        from langchain_google_genai import ChatGoogleGenerativeAI

        api_key = cfg.api_key or os.environ.get("GOOGLE_API_KEY")
        if not api_key:
            raise ValueError("Missing API key for provider 'google_genai'")
        return ChatGoogleGenerativeAI(
            model=target_model,
            google_api_key=api_key,
            temperature=cfg.temperature,
        )

    if cfg.provider == "openai":
        from langchain_openai import ChatOpenAI

        api_key = cfg.api_key or os.environ.get("OPENAI_API_KEY")
        if not api_key:
            raise ValueError("Missing API key for provider 'openai'")
        kwargs: dict[str, Any] = {
            "model": target_model,
            "api_key": api_key,
            "temperature": cfg.temperature,
        }
        if cfg.base_url:
            kwargs["base_url"] = cfg.base_url
        return ChatOpenAI(**kwargs)

    if cfg.provider == "anthropic":
        from langchain_anthropic import ChatAnthropic

        api_key = cfg.api_key or os.environ.get("ANTHROPIC_API_KEY")
        if not api_key:
            raise ValueError("Missing API key for provider 'anthropic'")
        return ChatAnthropic(
            model_name=target_model,
            anthropic_api_key=api_key,
            temperature=cfg.temperature,
        )

    raise ValueError(f"Unsupported LLM provider: {cfg.provider}")
