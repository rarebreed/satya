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

from fastapi import APIRouter, FastAPI, Header
from langgraph.checkpoint.memory import MemorySaver
from pydantic import Field

from satya_ml.agent_server import (
    BaseTurnEventPayload,
    SessionLockPool,
    resolve_turn_llm_config,
    verify_service_auth,
)
from satya_ml.ai_gm.models import redact_api_key
from satya_ml.ai_gm.telemetry import turn_telemetry_scope
from satya_ml.ai_player.graph import build_ai_player_graph
from satya_ml.campaign_config import get_pack_default_ai_player
from satya_ml.debug_dashboard import debug_router, is_debug_ui_enabled, run_observed_graph
from satya_ml.signaling_client import load_ai_gm_toml_config

logger = logging.getLogger("satya_ml.ai_player.server")

router = APIRouter()

_PLAYER_CHECKPOINTER = MemorySaver()
_PLAYER_CHAT_MODEL_OVERRIDE: Any | None = None
_PLAYER_LOCK_POOL = SessionLockPool()


def _get_player_lock(session_id: str, player_user_id: str) -> threading.Lock:
    return _PLAYER_LOCK_POOL.acquire_lock(f"{session_id}:{player_user_id}")


def clear_player_checkpointer_session(session_id: str) -> None:
    """Delete persisted LangGraph checkpoints for all player threads belonging to `session_id`."""
    prefix = f"{session_id}:player:"
    storage = getattr(_PLAYER_CHECKPOINTER, "storage", None)
    matching_threads = (
        [tid for tid in list(storage.keys()) if str(tid).startswith(prefix)]
        if isinstance(storage, dict)
        else []
    )
    if hasattr(_PLAYER_CHECKPOINTER, "delete_thread"):
        for tid in matching_threads:
            _PLAYER_CHECKPOINTER.delete_thread(tid)
        return
    if isinstance(storage, dict):
        for tid in matching_threads:
            storage.pop(tid, None)
    writes = getattr(_PLAYER_CHECKPOINTER, "writes", None)
    if isinstance(writes, dict):
        for key in [k for k in writes if isinstance(k, tuple) and k and str(k[0]).startswith(prefix)]:
            writes.pop(key, None)


def set_player_chat_model_override(model: Any | None) -> None:
    """Configure an optional chat model override for deterministic AI Player testing."""
    global _PLAYER_CHAT_MODEL_OVERRIDE
    _PLAYER_CHAT_MODEL_OVERRIDE = model


class AiPlayerTurnEventPayload(BaseTurnEventPayload):
    """Payload dispatched to `POST /internal/ai-player/events` to trigger an AI Player turn."""

    player_user_id: str = "ai-player-1"
    player_display_name: str = Field(
        default_factory=lambda: f"{get_pack_default_ai_player().get('character_name', 'Mateo Vance')} (AI Player)"
    )
    persona_prompt: str = ""
    character_sheet: dict[str, Any] | None = None
    combat_tracker: dict[str, Any] | None = None
    active_map: dict[str, Any] | None = None
    scene_prompt: str | None = None
    auto_trigger_gm: bool = True


def _verify_service_auth(authorization: str | None) -> None:
    verify_service_auth(authorization, detail="Unauthorized AI Player service token")


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

    llm_cfg, _file_cfg = resolve_turn_llm_config(
        payload,
        default_temperature=0.45,
        x_satya_provider_key=x_satya_provider_key,
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
            result, _run = run_observed_graph(
                graph,
                initial_state,
                {"configurable": {"thread_id": thread_id}},
                agent_kind="ai_player",
                session_id=payload.session_id,
                player_user_id=payload.player_user_id,
                trace_id=telemetry.trace_id,
                provider=str(llm_cfg.provider),
                model_id=str(llm_cfg.model_id),
                operating_mode="autonomous",
                trigger_summary=prompt_text,
                source="signaling",
                lock_getter=lambda sid=payload.session_id, uid=payload.player_user_id: _get_player_lock(sid, uid),
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
app.include_router(debug_router)


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
    parser.add_argument(
        "--standalone",
        "--offline",
        dest="standalone",
        action="store_true",
        help="Run standalone without satya-signaling (`SATYA_SIGNALING_URL=offline`)",
    )
    args = parser.parse_args(argv)
    if args.standalone:
        os.environ["SATYA_SIGNALING_URL"] = "offline"
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
    if is_debug_ui_enabled():
        logger.info("Debug Graph Dashboard available at http://%s:%d/debug", host, port)
    uvicorn.run(
        "satya_ml.ai_player.server:app",
        host=host,
        port=port,
        reload=False,
        log_level=uvicorn_log_level,
    )


if __name__ == "__main__":
    main()
