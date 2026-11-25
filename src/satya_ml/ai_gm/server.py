"""FastAPI service (`127.0.0.1:8090`) bridging `satya-signaling` to the LangGraph AI GM."""

from __future__ import annotations

import argparse
import logging
import os
import threading
import time
from typing import Any

from fastapi import FastAPI, Header
from langgraph.checkpoint.memory import MemorySaver
from pydantic import BaseModel, Field

from satya_ml.agent_server import (
    BaseTurnEventPayload,
    SessionLockPool,
    resolve_turn_llm_config,
    verify_service_auth,
)
from satya_ml.ai_gm.graph import build_ai_gm_graph
from satya_ml.ai_gm.models import redact_api_key
from satya_ml.ai_gm.ollama_runner import (
    ensure_ollama_running_and_logged,
    get_logs_dir,
    get_recent_ollama_logs,
)
from satya_ml.ai_gm.telemetry import (
    SatyaContextLogFormatter,
    SatyaTurnLogBufferHandler,
    turn_telemetry_scope,
)
from satya_ml.ai_gm.tools import set_session_scenario_director
from satya_ml.signaling_client import load_ai_gm_toml_config

logger = logging.getLogger("satya_ml.ai_gm.server")


def _normalize_operating_mode(raw: str) -> str:
    low = (raw or "").strip().lower()
    if "solo" in low:
        return "solo_co_gm"
    if "co_pilot" in low:
        return "co_pilot"
    return "autonomous"


def configure_ai_gm_logging(level_override: str | None = None) -> str:
    """Configure structured console logging, `data/logs/ai-gm.log` file persistence, and turn log buffering.

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
    has_file = False
    has_buffer = False
    for h in satya_logger.handlers:
        if isinstance(h, SatyaTurnLogBufferHandler):
            has_buffer = True
        elif isinstance(h, logging.FileHandler):
            has_file = True
            h.setLevel(numeric_level)
            h.setFormatter(SatyaContextLogFormatter())
        elif isinstance(h, logging.StreamHandler):
            has_stream = True
            h.setLevel(numeric_level)
            h.setFormatter(SatyaContextLogFormatter())

    if not has_stream:
        stream_handler = logging.StreamHandler()
        stream_handler.setLevel(numeric_level)
        stream_handler.setFormatter(SatyaContextLogFormatter())
        satya_logger.addHandler(stream_handler)

    if not has_file:
        try:
            ai_gm_log_path = get_logs_dir() / "ai-gm.log"
            file_handler = logging.FileHandler(ai_gm_log_path, encoding="utf-8")
            file_handler.setLevel(numeric_level)
            file_handler.setFormatter(SatyaContextLogFormatter())
            satya_logger.addHandler(file_handler)
        except OSError:
            pass

    if not has_buffer:
        satya_logger.addHandler(SatyaTurnLogBufferHandler())

    satya_logger.propagate = False

    if numeric_level <= logging.DEBUG and os.environ.get("SATYA_HTTPX_DEBUG") != "1":
        # Keep low-level HTTP socket frame noise at INFO unless explicitly requested
        logging.getLogger("httpcore").setLevel(logging.INFO)
        logging.getLogger("httpx").setLevel(logging.INFO)
    return logging.getLevelName(numeric_level).lower()


configure_ai_gm_logging()


from satya_ml.ai_player.server import router as ai_player_router
from satya_ml.debug_dashboard import debug_router, is_debug_ui_enabled, run_observed_graph

app = FastAPI(
    title="Satya AI GM LangGraph Service",
    version="0.1.0",
    description="Internal loopback bridge between satya-signaling (Rust) and LangGraph AI GM / AI Player.",
)
app.include_router(ai_player_router)
app.include_router(debug_router)

_SHARED_CHECKPOINTER = MemorySaver()
_CHAT_MODEL_OVERRIDE: Any | None = None
_SESSION_LOCK_POOL = SessionLockPool()


def clear_gm_checkpointer_session(session_id: str) -> None:
    """Delete persisted LangGraph checkpoints for `session_id` in `_SHARED_CHECKPOINTER`."""
    if hasattr(_SHARED_CHECKPOINTER, "delete_thread"):
        _SHARED_CHECKPOINTER.delete_thread(session_id)
        return
    storage = getattr(_SHARED_CHECKPOINTER, "storage", None)
    if isinstance(storage, dict):
        storage.pop(session_id, None)
    writes = getattr(_SHARED_CHECKPOINTER, "writes", None)
    if isinstance(writes, dict):
        for key in [k for k in writes if isinstance(k, tuple) and k and k[0] == session_id]:
            writes.pop(key, None)


def _get_session_lock(session_id: str) -> threading.Lock:
    """Return a per-session lock so concurrent player events serialize cleanly against the checkpointer."""
    return _SESSION_LOCK_POOL.acquire_lock(session_id)


def set_chat_model_override(model: Any | None) -> None:
    """Configure an optional chat model override for deterministic testing."""
    global _CHAT_MODEL_OVERRIDE
    _CHAT_MODEL_OVERRIDE = model


class AiGmTurnEventPayload(BaseTurnEventPayload):
    """Payload dispatched by `satya-signaling` (`AiGmTurnTriggerRequest`) to `POST /internal/ai-gm/events`."""

    session_mode: str = "TTRPG"
    operating_mode: str = "AUTONOMOUS"
    human_gm_user_id: str | None = None
    # Support both Rust `AiGmTurnTriggerRequest` field names (`trigger_event_id`, `trigger_actor`, `context_text`)
    # and shorthand field names (`event_id`, `actor`, `content`)
    trigger_kind: str = "CHAT"
    trigger_actor: dict[str, Any] | None = None
    actor: dict[str, Any] = Field(default_factory=dict)
    visibility: str = "PUBLIC"
    channel_context: str | None = None
    channel: str | None = None
    context_text: str | None = None
    roll_summary: str | None = None
    force_resolve_turn: bool = False
    persona_id: str | None = None
    provider_config: dict[str, Any] | None = None


class ScenarioBeatRequestPayload(BaseModel):
    """Payload for `POST /internal/ai-gm/scenario-beat` to trigger a proactive world beat."""

    session_id: str
    task_slug: str = "auto"
    custom_context: str = ""
    flow_mode: str | None = None
    campaign_premise: str | None = None
    planned_beats: list[str] | None = None
    player_desires: list[str] | None = None
    event_weights: dict[str, float] | None = None
    beat_probability_per_turn: float | None = None
    beat_interval_turns: int | None = None
    rng_seed: int | None = None
    operating_mode: str = "AUTONOMOUS"
    provider: str | None = None
    model_id: str | None = None


def _verify_service_auth(authorization: str | None) -> None:
    verify_service_auth(authorization, detail="Unauthorized AI GM service token")


@app.get("/health")
def health() -> dict[str, str]:
    """Liveness check for `satya-signaling` and `pixi run serve-ai-gm`."""
    return {"status": "ok", "service": "satya-ai-gm"}


@app.get("/internal/ai-gm/ollama-logs")
def get_ollama_logs_endpoint(
    lines: int = 100,
    high_signal_only: bool = False,
    authorization: str | None = Header(default=None),
) -> dict[str, Any]:
    """Return recent `ollama serve` logs collected into `data/logs/ollama.log` and the in-memory ring buffer."""
    _verify_service_auth(authorization)
    return get_recent_ollama_logs(limit=lines, high_signal_only=high_signal_only)


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

    llm_cfg, file_cfg = resolve_turn_llm_config(
        payload,
        default_temperature=0.35,
        x_satya_provider_key=x_satya_provider_key,
        provider_config=payload.provider_config,
    )
    p_cfg = payload.provider_config or {}
    effective_actor = payload.trigger_actor or payload.actor
    effective_content = payload.context_text if payload.context_text is not None else payload.content
    effective_event_id = payload.trigger_event_id or payload.event_id
    effective_lineage_id = (
        (x_satya_lineage_id if isinstance(x_satya_lineage_id, str) else None)
        or payload.lineage_id
        or effective_event_id
    )

    raw_lazy: bool | None
    if "lazy_tool_loading" in p_cfg:
        raw_lazy = bool(p_cfg.get("lazy_tool_loading"))
    elif os.environ.get("SATYA_AI_GM_LAZY_TOOLS", "").strip():
        raw_lazy = None
    else:
        raw_lazy = bool(file_cfg.get("lazy_tool_loading", True))
    graph = build_ai_gm_graph(
        llm_cfg,
        chat_model=_CHAT_MODEL_OVERRIDE,
        checkpointer=_SHARED_CHECKPOINTER,
        lazy_tool_loading=raw_lazy,
    )

    op_mode = _normalize_operating_mode(payload.operating_mode)
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
                "trigger_kind": payload.trigger_kind,
                "actor": effective_actor,
                "channel": payload.channel or payload.visibility,
                "channel_context": payload.channel_context,
                "content": effective_content,
                "roll_summary": payload.roll_summary,
                "force_resolve_turn": payload.force_resolve_turn,
                "operating_mode": op_mode,
                "human_gm_user_id": payload.human_gm_user_id,
                "trace_id": telemetry.trace_id,
                "parent_span_id": telemetry.parent_span_id,
                "lineage_id": effective_lineage_id,
            },
            "tool_invocations": [],
        }

        with _get_session_lock(payload.session_id):
            result, _run = run_observed_graph(
                graph,
                initial_state,
                {"configurable": {"thread_id": payload.session_id}},
                agent_kind="ai_gm",
                session_id=payload.session_id,
                trace_id=telemetry.trace_id,
                provider=str(llm_cfg.provider),
                model_id=str(llm_cfg.model_id),
                operating_mode=op_mode,
                trigger_summary=str(effective_content or ""),
                source="signaling",
                lock_getter=lambda sid=payload.session_id: _get_session_lock(sid),
            )
        elapsed_ms = int((time.perf_counter() - turn_t0) * 1000.0)
        tool_count = len(result.get("tool_invocations") or [])
        final_reply = result.get("final_reply") or ""
        logger.info(
            "AI GM turn completed in %d ms: channel=%s gate=%s tools_called=%d reply_chars=%d",
            elapsed_ms,
            result.get("final_channel"),
            result.get("party_gate_status", "bypass"),
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
            "party_gate_status": result.get("party_gate_status", "bypass"),
            "awaiting_user_ids": list(result.get("awaiting_user_ids") or []),
            "awaiting_display_names": list(result.get("awaiting_display_names") or []),
            "npc_turn_summaries": list(result.get("npc_turn_summaries") or []),
            "final_channel": result.get("final_channel"),
            "final_reply": result.get("final_reply"),
            "active_director_mandate": result.get("active_director_mandate", ""),
            "scenario_director": result.get("scenario_director"),
            "tool_invocations_count": tool_count,
            "turn_logs_count": len(telemetry.log_buffer),
        }


@app.post("/internal/ai-gm/scenario-beat")
def handle_scenario_beat(
    payload: ScenarioBeatRequestPayload,
    authorization: str | None = Header(default=None),
    x_satya_provider_key: str | None = Header(default=None),
    traceparent: str | None = Header(default=None),
) -> dict[str, Any]:
    """Proactively trigger a Scenario Director world beat for `payload.session_id`."""
    _verify_service_auth(authorization)
    updates: dict[str, Any] = {}
    if payload.flow_mode is not None:
        updates["flow_mode"] = payload.flow_mode
    if payload.campaign_premise is not None:
        updates["campaign_premise"] = payload.campaign_premise
    if payload.planned_beats is not None:
        updates["planned_beats"] = payload.planned_beats
    if payload.player_desires is not None:
        updates["player_desires"] = payload.player_desires
    if payload.event_weights is not None:
        updates["event_weights"] = payload.event_weights
    if payload.beat_probability_per_turn is not None:
        updates["beat_probability_per_turn"] = payload.beat_probability_per_turn
    if payload.beat_interval_turns is not None:
        updates["beat_interval_turns"] = payload.beat_interval_turns
    if updates:
        set_session_scenario_director(payload.session_id, updates)

    llm_cfg, file_cfg = resolve_turn_llm_config(
        payload,
        default_temperature=0.35,
        x_satya_provider_key=x_satya_provider_key,
    )
    raw_lazy: bool | None = (
        None
        if os.environ.get("SATYA_AI_GM_LAZY_TOOLS", "").strip()
        else bool(file_cfg.get("lazy_tool_loading", True))
    )
    graph = build_ai_gm_graph(
        llm_cfg,
        chat_model=_CHAT_MODEL_OVERRIDE,
        checkpointer=_SHARED_CHECKPOINTER,
        lazy_tool_loading=raw_lazy,
    )
    op_mode = _normalize_operating_mode(payload.operating_mode)
    with turn_telemetry_scope(payload.session_id, traceparent=traceparent) as telemetry:
        initial_state = {
            "session_id": payload.session_id,
            "session_mode": "TTRPG",
            "operating_mode": op_mode,
            "trigger_event": {
                "trigger_kind": "SCENARIO_BEAT",
                "force_scenario_beat": True,
                "task_slug": payload.task_slug,
                "custom_context": payload.custom_context,
                "rng_seed": payload.rng_seed,
                "actor": {"user_id": "scenario-director", "display_name": "Scenario Director"},
                "channel": "PUBLIC",
                "content": payload.custom_context or f"Trigger proactive scenario beat ({payload.task_slug})",
                "operating_mode": op_mode,
                "trace_id": telemetry.trace_id,
            },
            "tool_invocations": [],
        }
        with _get_session_lock(payload.session_id):
            result, _run = run_observed_graph(
                graph,
                initial_state,
                {"configurable": {"thread_id": payload.session_id}},
                agent_kind="ai_gm",
                session_id=payload.session_id,
                trace_id=telemetry.trace_id,
                provider=str(llm_cfg.provider),
                model_id=str(llm_cfg.model_id),
                operating_mode=op_mode,
                trigger_summary=f"SCENARIO_BEAT ({payload.task_slug}): {payload.custom_context}"[:240],
                source="signaling",
                lock_getter=lambda sid=payload.session_id: _get_session_lock(sid),
            )
        return {
            "status": "ok",
            "session_id": payload.session_id,
            "trace_id": telemetry.trace_id,
            "operating_mode": result.get("operating_mode", op_mode),
            "final_channel": result.get("final_channel"),
            "final_reply": result.get("final_reply"),
            "active_director_mandate": result.get("active_director_mandate", ""),
            "scenario_director": result.get("scenario_director"),
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
    parser.add_argument(
        "--restart-ollama",
        action="store_true",
        help="Restart any existing `ollama` process and launch a managed `ollama serve` capturing stdout/stderr into data/logs/ollama.log",
    )
    parser.add_argument(
        "--no-manage-ollama",
        action="store_true",
        help="Disable automatic `ollama serve` detection/startup and log collection",
    )
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

    host = os.environ.get("SATYA_AI_GM_HOST", "127.0.0.1")
    port = int(os.environ.get("SATYA_AI_GM_PORT", "8090"))
    file_cfg = load_ai_gm_toml_config()
    default_provider = str(file_cfg.get("default_provider", "ollama"))
    if default_provider == "ollama" and not args.no_manage_ollama:
        ollama_url = str(file_cfg.get("ollama_base_url") or os.environ.get("OLLAMA_HOST") or "http://127.0.0.1:11434")
        ensure_ollama_running_and_logged(
            base_url=ollama_url,
            file_cfg=file_cfg,
            restart_if_running=bool(args.restart_ollama),
        )
    logger.info(
        "Starting Satya AI GM server on http://%s:%d (log_level=%s, default_model=%s/%s)",
        host,
        port,
        uvicorn_log_level.upper(),
        default_provider,
        file_cfg.get("reasoning_model", "gemma4:12b"),
    )
    if is_debug_ui_enabled():
        logger.info("Debug Graph Dashboard available at http://%s:%d/debug", host, port)
    uvicorn.run(
        "satya_ml.ai_gm.server:app",
        host=host,
        port=port,
        reload=False,
        log_level=uvicorn_log_level,
    )


if __name__ == "__main__":
    main()
