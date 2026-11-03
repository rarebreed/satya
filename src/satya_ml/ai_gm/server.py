"""FastAPI service (`127.0.0.1:8090`) bridging `satya-signaling` to the LangGraph AI GM."""

from __future__ import annotations

import argparse
import logging
import os
import time
from typing import Any

from fastapi import FastAPI, Header, HTTPException
from langgraph.checkpoint.memory import MemorySaver
from pydantic import BaseModel, Field

from satya_ml.ai_gm.graph import build_ai_gm_graph
from satya_ml.ai_gm.models import SessionLlmConfig, redact_api_key
from satya_ml.ai_gm.telemetry import (
    SatyaContextLogFormatter,
    SatyaTurnLogBufferHandler,
    turn_telemetry_scope,
)
from satya_ml.ai_gm.tools import _gm_service_token, load_ai_gm_toml_config

logger = logging.getLogger("satya_ml.ai_gm.server")


def configure_ai_gm_logging(level_override: str | None = None) -> str:
    """Configure structured console logging and turn log buffering for the Satya AI GM LangGraph service.

    Precedence:
    1. Explicit `level_override` argument (e.g. from `--log-level` or `--debug`)
    2. `SATYA_AI_GM_LOG_LEVEL` or `LOG_LEVEL` environment variable
    3. `[ai_gm].log_level` in `config/ai.toml` (defaults to `"info"`)
    """
    file_cfg = load_ai_gm_toml_config()
    raw_level = (
        level_override
        or os.environ.get("SATYA_AI_GM_LOG_LEVEL")
        or os.environ.get("LOG_LEVEL")
        or str(file_cfg.get("log_level") or "INFO")
    ).strip().upper()
    numeric_level = getattr(logging, raw_level, logging.INFO)

    satya_logger = logging.getLogger("satya_ml")
    satya_logger.setLevel(min(numeric_level, logging.INFO))
    has_stream = False
    has_buffer = False
    for h in satya_logger.handlers:
        if isinstance(h, SatyaTurnLogBufferHandler):
            has_buffer = True
        elif isinstance(h, logging.StreamHandler):
            has_stream = True
            h.setLevel(numeric_level)
            h.setFormatter(SatyaContextLogFormatter())

    if not has_stream:
        stream_handler = logging.StreamHandler()
        stream_handler.setLevel(numeric_level)
        stream_handler.setFormatter(SatyaContextLogFormatter())
        satya_logger.addHandler(stream_handler)

    if not has_buffer:
        satya_logger.addHandler(SatyaTurnLogBufferHandler())

    satya_logger.propagate = False

    if numeric_level <= logging.DEBUG and os.environ.get("SATYA_HTTPX_DEBUG") != "1":
        # Keep low-level HTTP socket frame noise at INFO unless explicitly requested
        logging.getLogger("httpcore").setLevel(logging.INFO)
        logging.getLogger("httpx").setLevel(logging.INFO)
    return logging.getLevelName(numeric_level).lower()


configure_ai_gm_logging()


app = FastAPI(
    title="Satya AI GM LangGraph Service",
    version="0.1.0",
    description="Internal loopback bridge between satya-signaling (Rust) and LangGraph AI GM.",
)

_SHARED_CHECKPOINTER = MemorySaver()
_CHAT_MODEL_OVERRIDE: Any | None = None


def set_chat_model_override(model: Any | None) -> None:
    """Configure an optional chat model override for deterministic testing."""
    global _CHAT_MODEL_OVERRIDE
    _CHAT_MODEL_OVERRIDE = model


class AiGmTurnEventPayload(BaseModel):
    """Payload dispatched by `satya-signaling` (`AiGmTurnTriggerRequest`) to `POST /internal/ai-gm/events`."""

    session_id: str
    session_mode: str = "TTRPG"
    operating_mode: str = "AUTONOMOUS"
    human_gm_user_id: str | None = None
    # Support both Rust `AiGmTurnTriggerRequest` field names (`trigger_event_id`, `trigger_actor`, `context_text`)
    # and shorthand field names (`event_id`, `actor`, `content`)
    trigger_event_id: str | None = None
    event_id: str | None = None
    trigger_kind: str = "CHAT"
    trigger_actor: dict[str, Any] | None = None
    actor: dict[str, Any] = Field(default_factory=dict)
    visibility: str = "PUBLIC"
    channel: str | None = None
    context_text: str | None = None
    content: str | None = None
    roll_summary: str | None = None
    roster: list[dict[str, Any]] = Field(default_factory=list)
    provider: str | None = None
    model_id: str | None = None
    vision_model_id: str | None = None
    temperature: float | None = None
    persona_id: str | None = None
    base_url: str | None = None
    provider_config: dict[str, Any] | None = None
    trace_id: str | None = None
    parent_span_id: str | None = None
    lineage_id: str | None = None


def _verify_service_auth(authorization: str | None) -> None:
    expected = f"Bearer {_gm_service_token()}"
    if authorization != expected:
        raise HTTPException(status_code=401, detail="Unauthorized GM bridge token")


@app.get("/health")
def health() -> dict[str, str]:
    """Liveness check for `satya-signaling` and `pixi run serve-ai-gm`."""
    return {"status": "ok", "service": "satya-ai-gm"}


@app.post("/internal/ai-gm/events")
def handle_ai_gm_event(
    payload: AiGmTurnEventPayload,
    authorization: str | None = Header(default=None),
    x_satya_provider_key: str | None = Header(default=None),
    traceparent: str | None = Header(default=None),
    x_satya_lineage_id: str | None = Header(default=None),
) -> dict[str, Any]:
    """Execute one LangGraph AI GM turn for `payload.session_id`."""
    _verify_service_auth(authorization)

    p_cfg = payload.provider_config or {}
    file_cfg = load_ai_gm_toml_config()
    provider = str(
        payload.provider
        or p_cfg.get("provider")
        or file_cfg.get("default_provider")
        or "ollama"
    )
    model_id = str(
        payload.model_id
        or p_cfg.get("model_id")
        or file_cfg.get("reasoning_model")
        or "gemma4:12b"
    )
    vision_model_id = (
        payload.vision_model_id
        or p_cfg.get("vision_model_id")
        or file_cfg.get("vision_model")
        or "gemma4:12b"
    )
    temperature = float(
        payload.temperature
        if payload.temperature is not None
        else (
            p_cfg.get("temperature")
            if p_cfg.get("temperature") is not None
            else (file_cfg.get("temperature") if file_cfg.get("temperature") is not None else 0.35)
        )
    )
    base_url = (
        payload.base_url
        or p_cfg.get("base_url")
        or file_cfg.get("ollama_base_url")
        or "http://127.0.0.1:11434"
    )

    llm_cfg = SessionLlmConfig(
        provider=provider,  # type: ignore[arg-type]
        model_id=model_id,
        vision_model_id=vision_model_id,
        temperature=temperature,
        base_url=base_url,
        api_key=x_satya_provider_key,
    )

    graph = build_ai_gm_graph(
        llm_cfg,
        chat_model=_CHAT_MODEL_OVERRIDE,
        checkpointer=_SHARED_CHECKPOINTER,
    )

    op_mode = "co_pilot" if "co_pilot" in payload.operating_mode.lower() else "autonomous"
    effective_actor = payload.trigger_actor or payload.actor
    effective_content = payload.context_text if payload.context_text is not None else payload.content
    effective_event_id = payload.trigger_event_id or payload.event_id
    effective_lineage_id = x_satya_lineage_id or payload.lineage_id or effective_event_id
    actor_display = (
        (effective_actor or {}).get("display_name")
        or (effective_actor or {}).get("username")
        or "Unknown"
    )

    with turn_telemetry_scope(
        payload.session_id,
        traceparent=traceparent,
        trace_id=payload.trace_id,
        parent_span_id=payload.parent_span_id,
        lineage_id=effective_lineage_id,
    ) as telemetry:
        logger.info(
            "AI GM turn received: mode=%s kind=%s vis=%s actor='%s' model=%s/%s",
            op_mode,
            payload.trigger_kind,
            payload.channel or payload.visibility,
            actor_display,
            llm_cfg.provider,
            llm_cfg.model_id,
        )
        logger.debug(
            "Trigger event_id=%s content=%r roster_size=%d",
            effective_event_id,
            effective_content,
            len(payload.roster),
        )
        turn_t0 = time.perf_counter()

        initial_state = {
            "session_id": payload.session_id,
            "session_mode": payload.session_mode,
            "operating_mode": op_mode,
            "human_gm_user_id": payload.human_gm_user_id,
            "roster": payload.roster,
            "trigger_event": {
                "event_id": effective_event_id,
                "actor": effective_actor,
                "channel": payload.channel or payload.visibility,
                "content": effective_content,
                "roll_summary": payload.roll_summary,
                "operating_mode": op_mode,
                "human_gm_user_id": payload.human_gm_user_id,
                "trace_id": telemetry.trace_id,
                "parent_span_id": telemetry.parent_span_id,
                "lineage_id": effective_lineage_id,
            },
            "tool_invocations": [],
        }

        result = graph.invoke(
            initial_state,
            config={"configurable": {"thread_id": payload.session_id}},
        )
        elapsed_ms = int((time.perf_counter() - turn_t0) * 1000.0)
        tool_count = len(result.get("tool_invocations") or [])
        final_reply = result.get("final_reply") or ""
        logger.info(
            "AI GM turn completed in %d ms: channel=%s tools_called=%d reply_chars=%d",
            elapsed_ms,
            result.get("final_channel"),
            tool_count,
            len(final_reply),
        )

        return {
            "status": "ok",
            "session_id": payload.session_id,
            "trace_id": telemetry.trace_id,
            "operating_mode": result.get("operating_mode", op_mode),
            "provider": llm_cfg.provider,
            "model_id": llm_cfg.model_id,
            "api_key_status": redact_api_key(x_satya_provider_key),
            "final_channel": result.get("final_channel"),
            "final_reply": result.get("final_reply"),
            "tool_invocations_count": tool_count,
            "turn_logs_count": len(telemetry.log_buffer),
        }


def main(argv: list[str] | None = None) -> None:
    """Run the FastAPI AI GM server via uvicorn on `127.0.0.1:8090`."""
    import uvicorn

    parser = argparse.ArgumentParser(description="Satya AI GM FastAPI + LangGraph server")
    parser.add_argument(
        "--debug",
        action="store_true",
        help="Enable verbose DEBUG logging for LangGraph nodes, LLM calls, and tool executions",
    )
    parser.add_argument(
        "--log-level",
        default=None,
        help="Explicit log level (debug, info, warning, error)",
    )
    args = parser.parse_args(argv)
    level_override = "DEBUG" if args.debug else args.log_level
    uvicorn_log_level = configure_ai_gm_logging(level_override)

    host = os.environ.get("SATYA_AI_GM_HOST", "127.0.0.1")
    port = int(os.environ.get("SATYA_AI_GM_PORT", "8090"))
    file_cfg = load_ai_gm_toml_config()
    logger.info(
        "Starting Satya AI GM server on http://%s:%d (log_level=%s, default_model=%s/%s)",
        host,
        port,
        uvicorn_log_level.upper(),
        file_cfg.get("default_provider", "ollama"),
        file_cfg.get("reasoning_model", "gemma4:12b"),
    )
    uvicorn.run(
        "satya_ml.ai_gm.server:app",
        host=host,
        port=port,
        reload=False,
        log_level=uvicorn_log_level,
    )


if __name__ == "__main__":
    main()
