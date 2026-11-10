"""FastAPI router and standalone microservice (`127.0.0.1:8091`) for the Satya AI Player Agent (`satya_ml.ai_player`).

Can run either:
1. Mounted inside `satya_ml.ai_gm.server:app` (`/internal/ai-player/events`) with isolated checkpointer and lock state, or
2. As an independent process (`pixi run serve-ai-player` on port `8091` or on another LAN PC via `SATYA_AI_PLAYER_HOST=0.0.0.0`).
"""

from __future__ import annotations

import argparse
import logging
import os
import threading
import time
from typing import Any

from fastapi import APIRouter, FastAPI, Header, HTTPException
from langgraph.checkpoint.memory import MemorySaver
from pydantic import BaseModel, Field

from satya_ml.ai_gm.models import SessionLlmConfig, redact_api_key
from satya_ml.ai_gm.telemetry import turn_telemetry_scope
from satya_ml.ai_gm.tools import _gm_service_token, load_ai_gm_toml_config
from satya_ml.ai_player.graph import build_ai_player_graph

logger = logging.getLogger("satya_ml.ai_player.server")

router = APIRouter()

_PLAYER_CHECKPOINTER = MemorySaver()
_PLAYER_CHAT_MODEL_OVERRIDE: Any | None = None
_PLAYER_LOCKS_GUARD = threading.Lock()
_PLAYER_TURN_LOCKS: dict[str, threading.Lock] = {}


def _get_player_lock(session_id: str, player_user_id: str) -> threading.Lock:
    key = f"{session_id}:{player_user_id}"
    with _PLAYER_LOCKS_GUARD:
        lock = _PLAYER_TURN_LOCKS.get(key)
        if lock is None:
            lock = threading.Lock()
            _PLAYER_TURN_LOCKS[key] = lock
        return lock


def set_player_chat_model_override(model: Any | None) -> None:
    """Configure an optional chat model override for deterministic AI Player testing."""
    global _PLAYER_CHAT_MODEL_OVERRIDE
    _PLAYER_CHAT_MODEL_OVERRIDE = model


class AiPlayerTurnEventPayload(BaseModel):
    """Payload dispatched to `POST /internal/ai-player/events` to trigger an AI Player turn."""

    session_id: str
    player_user_id: str = "ai-player-1"
    player_display_name: str = "Kaelen Voss (AI Player)"
    persona_prompt: str = ""
    character_sheet: dict[str, Any] | None = None
    combat_tracker: dict[str, Any] | None = None
    active_map: dict[str, Any] | None = None
    roster: list[dict[str, Any]] = Field(default_factory=list)
    trigger_event_id: str | None = None
    event_id: str | None = None
    content: str | None = None
    scene_prompt: str | None = None
    auto_trigger_gm: bool = True
    provider: str | None = None
    model_id: str | None = None
    vision_model_id: str | None = None
    temperature: float | None = None
    base_url: str | None = None
    trace_id: str | None = None
    parent_span_id: str | None = None
    lineage_id: str | None = None


def _verify_service_auth(authorization: str | None) -> None:
    expected = f"Bearer {_gm_service_token()}"
    if authorization != expected:
        raise HTTPException(status_code=401, detail="Unauthorized AI Player bridge token")


@router.post("/internal/ai-player/events")
def handle_ai_player_event(
    payload: AiPlayerTurnEventPayload,
    authorization: str | None = Header(default=None),
    x_satya_provider_key: str | None = Header(default=None),
    traceparent: str | None = Header(default=None),
    x_satya_lineage_id: str | None = Header(default=None),
) -> dict[str, Any]:
    """Execute one isolated LangGraph AI Player turn for `(payload.session_id, payload.player_user_id)`."""
    _verify_service_auth(authorization)

    file_cfg = load_ai_gm_toml_config()
    provider = str(payload.provider or file_cfg.get("default_provider") or "ollama")
    model_id = str(payload.model_id or file_cfg.get("reasoning_model") or "gemma4:12b")
    vision_model_id = str(payload.vision_model_id or file_cfg.get("vision_model") or "gemma4:12b")
    temperature = float(
        payload.temperature
        if payload.temperature is not None
        else (file_cfg.get("temperature") if file_cfg.get("temperature") is not None else 0.45)
    )
    base_url = str(
        payload.base_url
        or file_cfg.get("ollama_base_url")
        or "http://127.0.0.1:11434"
    )
    num_ctx = int(file_cfg.get("num_ctx") or 16384)
    num_predict = int(file_cfg.get("num_predict") or 2048)
    raw_reasoning = file_cfg.get("reasoning", False)
    reasoning = bool(raw_reasoning) if raw_reasoning is not None else False
    raw_provider_key = x_satya_provider_key if isinstance(x_satya_provider_key, str) else None

    llm_cfg = SessionLlmConfig(
        provider=provider,  # type: ignore[arg-type]
        model_id=model_id,
        vision_model_id=vision_model_id,
        temperature=temperature,
        base_url=base_url,
        num_ctx=num_ctx,
        num_predict=num_predict,
        reasoning=reasoning,
        api_key=raw_provider_key,
    )

    graph = build_ai_player_graph(
        llm_cfg,
        chat_model=_PLAYER_CHAT_MODEL_OVERRIDE,
        checkpointer=_PLAYER_CHECKPOINTER,
    )

    effective_event_id = payload.trigger_event_id or payload.event_id
    effective_lineage_id = (
        (x_satya_lineage_id if isinstance(x_satya_lineage_id, str) else None)
        or payload.lineage_id
        or effective_event_id
    )
    prompt_text = (
        payload.content
        if payload.content is not None
        else (payload.scene_prompt or "Declare your character's next action.")
    )
    thread_id = f"{payload.session_id}:player:{payload.player_user_id}"

    with turn_telemetry_scope(
        payload.session_id,
        traceparent=traceparent,
        trace_id=payload.trace_id,
        parent_span_id=payload.parent_span_id,
        lineage_id=effective_lineage_id,
    ) as telemetry:
        t0 = time.perf_counter()
        initial_state = {
            "session_id": payload.session_id,
            "player_user_id": payload.player_user_id,
            "player_display_name": payload.player_display_name,
            "persona_prompt": payload.persona_prompt,
            "roster": payload.roster,
            "character_sheet": payload.character_sheet,
            "combat_tracker": payload.combat_tracker,
            "active_map": payload.active_map,
            "trigger_event": {
                "event_id": effective_event_id,
                "player_user_id": payload.player_user_id,
                "player_display_name": payload.player_display_name,
                "persona_prompt": payload.persona_prompt,
                "character_sheet": payload.character_sheet,
                "combat_tracker": payload.combat_tracker,
                "active_map": payload.active_map,
                "roster": payload.roster,
                "content": prompt_text,
                "auto_trigger_gm": payload.auto_trigger_gm,
            },
            "tool_invocations": [],
            "roll_results": [],
        }

        with _get_player_lock(payload.session_id, payload.player_user_id):
            result = graph.invoke(
                initial_state,
                config={"configurable": {"thread_id": thread_id}},
            )

        elapsed_ms = int((time.perf_counter() - t0) * 1000.0)
        final_reply = result.get("final_reply") or ""
        roll_results = list(result.get("roll_results") or [])
        tool_count = len(result.get("tool_invocations") or [])

        logger.info(
            "AI Player turn completed in %d ms: player='%s' (%s) tools=%d rolls=%d reply_chars=%d",
            elapsed_ms,
            payload.player_display_name,
            payload.player_user_id,
            tool_count,
            len(roll_results),
            len(final_reply),
        )

        return {
            "status": "ok",
            "session_id": payload.session_id,
            "player_user_id": payload.player_user_id,
            "player_display_name": payload.player_display_name,
            "trace_id": telemetry.trace_id,
            "provider": llm_cfg.provider,
            "model_id": llm_cfg.model_id,
            "api_key_status": redact_api_key(x_satya_provider_key),
            "final_channel": result.get("final_channel", "IN_CHARACTER"),
            "final_reply": final_reply,
            "roll_results": roll_results,
            "tool_invocations_count": tool_count,
            "turn_logs_count": len(telemetry.log_buffer),
        }


app = FastAPI(
    title="Satya AI Player LangGraph Service",
    version="0.1.0",
    description="Isolated AI Player LangGraph service for Satya multi-player and combat testing.",
)
app.include_router(router)


@app.get("/health")
def health() -> dict[str, str]:
    """Liveness check for standalone `pixi run serve-ai-player`."""
    return {"status": "ok", "service": "satya-ai-player"}


def main(argv: list[str] | None = None) -> None:
    """Run the standalone FastAPI AI Player service via uvicorn on `127.0.0.1:8091`."""
    import uvicorn
    from satya_ml.ai_gm.server import configure_ai_gm_logging

    parser = argparse.ArgumentParser(description="Satya AI Player FastAPI + LangGraph server")
    parser.add_argument("--debug", action="store_true", help="Enable verbose DEBUG logging")
    parser.add_argument("--log-level", default=None, help="Explicit log level")
    args = parser.parse_args(argv)
    level_override = "DEBUG" if args.debug else args.log_level
    uvicorn_log_level = configure_ai_gm_logging(level_override)

    host = os.environ.get("SATYA_AI_PLAYER_HOST", "127.0.0.1")
    port = int(os.environ.get("SATYA_AI_PLAYER_PORT", "8091"))
    file_cfg = load_ai_gm_toml_config()
    logger.info(
        "Starting Satya AI Player server on http://%s:%d (log_level=%s, default_model=%s/%s)",
        host,
        port,
        uvicorn_log_level.upper(),
        file_cfg.get("default_provider", "ollama"),
        file_cfg.get("reasoning_model", "gemma4:12b"),
    )
    uvicorn.run(
        "satya_ml.ai_player.server:app",
        host=host,
        port=port,
        reload=False,
        log_level=uvicorn_log_level,
    )


if __name__ == "__main__":
    main()
