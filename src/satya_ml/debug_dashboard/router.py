"""FastAPI router (`/debug`) for the live LangGraph Dashboard, Breakpoint Debugger & Scenario Workbench."""

from __future__ import annotations

import asyncio
from collections.abc import AsyncIterator
import json
import os
from pathlib import Path
from typing import Any, Final

from fastapi import APIRouter, Depends, HTTPException
from fastapi.responses import HTMLResponse, JSONResponse, Response, StreamingResponse
from pydantic import BaseModel, Field

from satya_ml.ai_gm.ollama_runner import get_recent_ollama_logs
from satya_ml.debug_dashboard.hub import (
    GRAPH_HUB,
    abort_paused_run,
    extract_graph_topology,
    resume_observed_run,
    to_json_safe,
)
from satya_ml.debug_dashboard.scenarios import (
    BUILTIN_SCENARIOS,
    ScenarioSpec,
    invoke_gm_debug_turn,
    invoke_player_debug_turn,
    invoke_scenario_beat_debug_turn,
    run_scenario,
    seed_offline_session,
)
from satya_ml.session_store import GM_SESSION_REGISTRY, PLAYER_SESSION_REGISTRY
from satya_ml.signaling_client import load_ai_gm_toml_config, signaling_base_url

_LOOPBACK_HOSTS: Final[frozenset[str]] = frozenset({"127.0.0.1", "localhost", "::1"})
_STATIC_DIR: Final[Path] = Path(__file__).resolve().parent / "static"
_STATIC_ALLOWLIST: Final[dict[str, tuple[str, str]]] = {
    "styles.css": ("styles.css", "text/css; charset=utf-8"),
    "app.js": ("app.js", "application/javascript; charset=utf-8"),
    "debug-brain.svg": ("debug-brain.svg", "image/svg+xml; charset=utf-8"),
}

router = APIRouter(prefix="/debug", tags=["debug-dashboard"])


def is_debug_ui_enabled() -> bool:
    """Return True when `/debug` is permitted (loopback hosts by default, or `SATYA_AI_DEBUG_UI=1`)."""
    raw_flag = os.environ.get("SATYA_AI_DEBUG_UI", "").strip().lower()
    if raw_flag in {"0", "false", "off", "no"}:
        return False
    if raw_flag in {"1", "true", "on", "yes"}:
        return True
    gm_host = os.environ.get("SATYA_AI_GM_HOST", "127.0.0.1").strip().lower()
    player_host = os.environ.get("SATYA_AI_PLAYER_HOST", "127.0.0.1").strip().lower()
    return gm_host in _LOOPBACK_HOSTS and player_host in _LOOPBACK_HOSTS


def _require_debug_enabled() -> None:
    if not is_debug_ui_enabled():
        raise HTTPException(
            status_code=403,
            detail="Debug UI is disabled on non-loopback hosts (set SATYA_AI_DEBUG_UI=1 to enable).",
        )


class DebugGmInvokePayload(BaseModel):
    session_id: str = "debug-session-1"
    prompt: str
    actor_user_id: str = "debug-player-1"
    actor_display_name: str = "Mateo Vance"
    operating_mode: str = "AUTONOMOUS"
    channel_context: str | None = None
    roll_summary: str | None = None
    roster: list[dict[str, Any]] = Field(default_factory=list)
    force_resolve_turn: bool = False
    offline_mode: bool = True
    provider: str | None = None
    model_id: str | None = None
    temperature: float | None = None
    interrupt_before: list[str] = Field(default_factory=list)
    mock_reply: str | None = None


class DebugPlayerInvokePayload(BaseModel):
    session_id: str = "debug-session-1"
    scene_prompt: str
    player_user_id: str = "ai-player-1"
    player_display_name: str | None = None
    persona_prompt: str = ""
    roster: list[dict[str, Any]] = Field(default_factory=list)
    offline_mode: bool = True
    provider: str | None = None
    model_id: str | None = None
    temperature: float | None = None
    interrupt_before: list[str] = Field(default_factory=list)
    mock_reply: str | None = None


class DebugScenarioBeatPayload(BaseModel):
    session_id: str = "debug-session-1"
    task_slug: str = "auto"
    custom_context: str = ""
    operating_mode: str = "AUTONOMOUS"
    offline_mode: bool = True
    provider: str | None = None
    model_id: str | None = None
    interrupt_before: list[str] = Field(default_factory=list)
    mock_reply: str | None = None


class DebugResumePayload(BaseModel):
    step_only: bool = False
    interrupt_before: list[str] | None = None
    state_patch: dict[str, Any] | None = None


class DebugScenarioRunPayload(BaseModel):
    preset_id: str | None = None
    scenario: ScenarioSpec | None = None
    provider: str | None = None
    model_id: str | None = None
    session_id: str | None = None
    offline_mode: bool | None = None


class DebugSeedSessionPayload(BaseModel):
    seed_config: dict[str, Any] = Field(default_factory=dict)


def _read_checkpoint_summaries(checkpointer: Any, thread_id: str, limit: int = 8) -> list[dict[str, Any]]:
    summaries: list[dict[str, Any]] = []
    if not hasattr(checkpointer, "list"):
        return summaries
    try:
        for idx, tup in enumerate(
            checkpointer.list({"configurable": {"thread_id": thread_id}})
        ):
            if idx >= limit:
                break
            ckpt = getattr(tup, "checkpoint", {}) or {}
            meta = getattr(tup, "metadata", {}) or {}
            pending_writes = getattr(tup, "pending_writes", None) or []
            summaries.append(
                {
                    "checkpoint_id": ckpt.get("id") if isinstance(ckpt, dict) else None,
                    "ts": ckpt.get("ts") if isinstance(ckpt, dict) else None,
                    "source": meta.get("source") if isinstance(meta, dict) else None,
                    "step": meta.get("step") if isinstance(meta, dict) else None,
                    "pending_writes_count": len(pending_writes),
                }
            )
    except Exception:
        return summaries
    return summaries


def _collect_session_state_snapshot(session_id: str) -> dict[str, Any]:
    """Read live GM & AI Player in-memory stores for `session_id` without mutating or lazily seeding."""
    from satya_ml.ai_gm.maps import get_session_map
    from satya_ml.ai_gm.scenario_director import _SESSION_SCENARIO_DIRECTORS
    from satya_ml.ai_gm.server import _SHARED_CHECKPOINTER
    from satya_ml.ai_gm.social_and_clocks import _SESSION_WORLD_LEDGERS
    from satya_ml.ai_gm.tools import (
        _SESSION_CHARACTERS,
        _SESSION_COMBAT_TRACKER,
        _SESSION_GM_DOCS,
        _SESSION_GM_MEMORIES,
        _SESSION_HUMAN_GMS,
        _SESSION_MODES,
        _SESSION_NPCS,
        _SESSION_PERSISTED_NPCS,
    )
    from satya_ml.ai_player.motivation_ledger import _PLAYER_MOTIVATION_LEDGERS
    from satya_ml.ai_player.server import _PLAYER_CHECKPOINTER
    from satya_ml.ai_player.tools import _PLAYER_CONTEXT_CACHE

    player_contexts = {
        uid: to_json_safe(ctx)
        for (sid, uid), ctx in list(_PLAYER_CONTEXT_CACHE.items())
        if sid == session_id
    }
    motivation_ledgers = {
        uid: to_json_safe(ledger)
        for (sid, uid), ledger in list(_PLAYER_MOTIVATION_LEDGERS.items())
        if sid == session_id
    }
    player_checkpoints = {
        uid: _read_checkpoint_summaries(_PLAYER_CHECKPOINTER, f"{session_id}:player:{uid}")
        for uid in sorted(set(player_contexts) | set(motivation_ledgers))
    }

    return {
        "session_id": session_id,
        "operating_mode": _SESSION_MODES.get(session_id, "autonomous"),
        "human_gm_user_id": _SESSION_HUMAN_GMS.get(session_id),
        "combat_tracker": to_json_safe(_SESSION_COMBAT_TRACKER.get(session_id)),
        "active_npcs": to_json_safe(_SESSION_NPCS.get(session_id, {})),
        "persisted_npcs": to_json_safe(_SESSION_PERSISTED_NPCS.get(session_id)),
        "active_map": to_json_safe(get_session_map(session_id)),
        "world_ledger": to_json_safe(_SESSION_WORLD_LEDGERS.get(session_id)),
        "scenario_director": to_json_safe(_SESSION_SCENARIO_DIRECTORS.get(session_id)),
        "characters": to_json_safe(_SESSION_CHARACTERS.get(session_id, [])),
        "gm_memories": to_json_safe(_SESSION_GM_MEMORIES.get(session_id, [])),
        "gm_docs": to_json_safe(_SESSION_GM_DOCS.get(session_id, [])),
        "player_contexts": player_contexts,
        "motivation_ledgers": motivation_ledgers,
        "checkpoints": {
            "ai_gm": _read_checkpoint_summaries(_SHARED_CHECKPOINTER, session_id),
            "ai_player": player_checkpoints,
        },
    }


def _collect_known_sessions() -> list[str]:
    from satya_ml.ai_gm.scenario_director import _SESSION_SCENARIO_DIRECTORS
    from satya_ml.ai_gm.tools import (
        _SESSION_CHARACTERS,
        _SESSION_COMBAT_TRACKER,
        _SESSION_NPCS,
    )
    from satya_ml.ai_player.tools import _PLAYER_CONTEXT_CACHE

    seen: dict[str, None] = {}
    for sid in GRAPH_HUB.list_known_sessions():
        seen.setdefault(sid, None)
    for store in (_SESSION_COMBAT_TRACKER, _SESSION_CHARACTERS, _SESSION_NPCS, _SESSION_SCENARIO_DIRECTORS):
        for sid in list(store.keys()):
            seen.setdefault(str(sid), None)
    for sid, _uid in list(_PLAYER_CONTEXT_CACHE.keys()):
        seen.setdefault(str(sid), None)
    return list(seen.keys())


@router.get("", response_class=HTMLResponse, dependencies=[Depends(_require_debug_enabled)])
@router.get("/", response_class=HTMLResponse, dependencies=[Depends(_require_debug_enabled)])
def debug_index() -> HTMLResponse:
    """Serve the zero-build Satya AI Debug & Graph Dashboard SPA."""
    html_path = _STATIC_DIR / "index.html"
    if not html_path.exists():
        raise HTTPException(status_code=404, detail="Debug UI index.html missing")
    return HTMLResponse(
        content=html_path.read_text(encoding="utf-8"),
        headers={"Cache-Control": "no-store"},
    )


@router.get("/static/{asset_name}", dependencies=[Depends(_require_debug_enabled)])
def debug_static_asset(asset_name: str) -> Response:
    """Serve allowlisted CSS/JS assets for the Debug SPA."""
    entry = _STATIC_ALLOWLIST.get(asset_name)
    if entry is None:
        raise HTTPException(status_code=404, detail="Static asset not found")
    rel_name, media_type = entry
    target = _STATIC_DIR / rel_name
    if not target.exists():
        raise HTTPException(status_code=404, detail="Static asset missing")
    return Response(
        content=target.read_bytes(),
        media_type=media_type,
        headers={"Cache-Control": "no-store"},
    )


@router.get("/api/topology", dependencies=[Depends(_require_debug_enabled)])
def get_debug_topology() -> dict[str, Any]:
    """Return canonical AI GM & AI Player graph topologies plus server defaults."""
    file_cfg = load_ai_gm_toml_config()
    trace_dir = GRAPH_HUB.trace_dir
    return {
        "graphs": extract_graph_topology(),
        "defaults": {
            "provider": str(file_cfg.get("default_provider") or "ollama"),
            "model_id": str(file_cfg.get("reasoning_model") or "gemma4:12b"),
            "temperature": float(file_cfg.get("temperature") or 0.35),
        },
        "signaling_url": signaling_base_url(),
        "offline_by_default": True,
        "trace_dir": str(trace_dir) if trace_dir is not None else None,
    }


@router.get("/api/runs", dependencies=[Depends(_require_debug_enabled)])
def list_debug_runs(
    session_id: str | None = None,
    agent_kind: str | None = None,
    limit: int = 50,
) -> dict[str, Any]:
    """Return recent observed AI GM / AI Player graph runs."""
    return {
        "runs": GRAPH_HUB.list_runs(session_id=session_id, agent_kind=agent_kind, limit=limit)
    }


@router.get("/api/runs/{run_id}", dependencies=[Depends(_require_debug_enabled)])
def get_debug_run(run_id: str) -> dict[str, Any]:
    """Return full step-by-step execution detail for `run_id`."""
    run = GRAPH_HUB.get_run(run_id)
    if run is None:
        raise HTTPException(status_code=404, detail=f"Run '{run_id}' not found")
    return {"run": run.to_detail_dict()}


@router.get("/api/runs/{run_id}/download", dependencies=[Depends(_require_debug_enabled)])
def download_debug_run(run_id: str) -> Response:
    """Download a redacted JSON trace file for `run_id`."""
    run = GRAPH_HUB.get_run(run_id)
    if run is None:
        raise HTTPException(status_code=404, detail=f"Run '{run_id}' not found")
    body = json.dumps(run.to_detail_dict(), indent=2)
    return Response(
        content=body,
        media_type="application/json",
        headers={
            "Content-Disposition": f'attachment; filename="satya-trace-{run.agent_kind}-{run.run_id}.json"'
        },
    )


@router.post("/api/runs/{run_id}/resume", dependencies=[Depends(_require_debug_enabled)])
def resume_debug_run(run_id: str, payload: DebugResumePayload) -> dict[str, Any]:
    """Resume or single-step a paused `interrupt_before` run from its checkpoint."""
    try:
        result, run = resume_observed_run(
            run_id,
            step_only=payload.step_only,
            interrupt_before=payload.interrupt_before,
            state_patch=payload.state_patch,
        )
    except KeyError as exc:
        raise HTTPException(status_code=404, detail=str(exc)) from exc
    return {
        "status": "ok",
        "run": run.to_detail_dict(),
        "final_reply": result.get("final_reply"),
    }


@router.post("/api/runs/{run_id}/abort", dependencies=[Depends(_require_debug_enabled)])
def abort_debug_run(run_id: str) -> dict[str, Any]:
    """Abort a paused `interrupt_before` run."""
    try:
        run = abort_paused_run(run_id)
    except KeyError as exc:
        raise HTTPException(status_code=404, detail=str(exc)) from exc
    return {"status": "ok", "run": run.to_detail_dict()}


@router.delete("/api/traces", dependencies=[Depends(_require_debug_enabled)])
def clear_debug_traces(delete_files: bool = False) -> dict[str, Any]:
    """Clear in-memory run history (and optionally persisted JSON trace files)."""
    cleared = GRAPH_HUB.clear_traces(delete_files=delete_files)
    return {"status": "ok", "cleared": cleared, "delete_files": delete_files}


@router.get("/api/stream", dependencies=[Depends(_require_debug_enabled)])
async def stream_debug_events() -> StreamingResponse:
    """Server-Sent Events stream pushing live node progress, deltas, pauses, and scenario updates."""
    sub_id, queue = GRAPH_HUB.subscribe()

    async def _event_generator() -> AsyncIterator[str]:
        try:
            hello = json.dumps(
                {
                    "type": "hello",
                    "data": {"runs": GRAPH_HUB.list_runs(limit=30)},
                }
            )
            yield f"data: {hello}\n\n"
            while True:
                try:
                    event = await asyncio.wait_for(queue.get(), timeout=15.0)
                    yield f"data: {json.dumps(event)}\n\n"
                except TimeoutError:
                    yield ": keepalive\n\n"
        finally:
            GRAPH_HUB.unsubscribe(sub_id)

    return StreamingResponse(
        _event_generator(),
        media_type="text/event-stream",
        headers={
            "Cache-Control": "no-cache",
            "Connection": "keep-alive",
            "X-Accel-Buffering": "no",
        },
    )


@router.get("/api/sessions", dependencies=[Depends(_require_debug_enabled)])
def list_debug_sessions() -> dict[str, Any]:
    """Return known active/observed `session_id`s."""
    return {"sessions": _collect_known_sessions()}


@router.get("/api/sessions/{session_id}/state", dependencies=[Depends(_require_debug_enabled)])
def get_debug_session_state(session_id: str) -> dict[str, Any]:
    """Inspect live in-memory GM & AI Player stores and checkpointer threads for `session_id`."""
    return _collect_session_state_snapshot(session_id)


@router.post("/api/sessions/{session_id}/seed", dependencies=[Depends(_require_debug_enabled)])
def seed_debug_session(session_id: str, payload: DebugSeedSessionPayload) -> dict[str, Any]:
    """Seed offline session stores for standalone debugging without `satya-signaling`."""
    summary = seed_offline_session(session_id, payload.seed_config)
    return {"status": "ok", **summary, "state": _collect_session_state_snapshot(session_id)}


@router.post("/api/sessions/{session_id}/reset", dependencies=[Depends(_require_debug_enabled)])
def reset_debug_session(session_id: str) -> dict[str, Any]:
    """Reset in-memory GM/Player stores and checkpointer threads for `session_id`."""
    from satya_ml.ai_gm.server import clear_gm_checkpointer_session
    from satya_ml.ai_player.server import clear_player_checkpointer_session

    GM_SESSION_REGISTRY.reset(session_id)
    PLAYER_SESSION_REGISTRY.reset(session_id)
    clear_gm_checkpointer_session(session_id)
    clear_player_checkpointer_session(session_id)
    GRAPH_HUB.publish_event("session_reset", {"session_id": session_id})
    return {"status": "ok", "session_id": session_id}


@router.post("/api/invoke/gm", dependencies=[Depends(_require_debug_enabled)])
def invoke_gm_from_debug(payload: DebugGmInvokePayload) -> dict[str, Any]:
    """Invoke the AI GM LangGraph directly from the Debug UI."""
    result, run = invoke_gm_debug_turn(
        session_id=payload.session_id,
        prompt=payload.prompt,
        actor_user_id=payload.actor_user_id,
        actor_display_name=payload.actor_display_name,
        operating_mode=payload.operating_mode,
        channel_context=payload.channel_context,
        roll_summary=payload.roll_summary,
        roster=payload.roster,
        force_resolve_turn=payload.force_resolve_turn,
        offline_mode=payload.offline_mode,
        provider=payload.provider,
        model_id=payload.model_id,
        temperature=payload.temperature,
        interrupt_before=payload.interrupt_before,
        mock_reply=payload.mock_reply,
        source="debug_ui",
    )
    return {
        "status": "ok",
        "run": run.to_detail_dict(),
        "final_reply": result.get("final_reply"),
        "party_gate_status": result.get("party_gate_status"),
    }


@router.post("/api/invoke/player", dependencies=[Depends(_require_debug_enabled)])
def invoke_player_from_debug(payload: DebugPlayerInvokePayload) -> dict[str, Any]:
    """Invoke the AI Player LangGraph directly from the Debug UI."""
    result, run = invoke_player_debug_turn(
        session_id=payload.session_id,
        scene_prompt=payload.scene_prompt,
        player_user_id=payload.player_user_id,
        player_display_name=payload.player_display_name,
        persona_prompt=payload.persona_prompt,
        roster=payload.roster,
        offline_mode=payload.offline_mode,
        provider=payload.provider,
        model_id=payload.model_id,
        temperature=payload.temperature,
        interrupt_before=payload.interrupt_before,
        mock_reply=payload.mock_reply,
        source="debug_ui",
    )
    return {
        "status": "ok",
        "run": run.to_detail_dict(),
        "final_reply": result.get("final_reply"),
    }


@router.post("/api/invoke/scenario-beat", dependencies=[Depends(_require_debug_enabled)])
def invoke_scenario_beat_from_debug(payload: DebugScenarioBeatPayload) -> dict[str, Any]:
    """Trigger a proactive Scenario Director beat directly from the Debug UI."""
    result, run = invoke_scenario_beat_debug_turn(
        session_id=payload.session_id,
        task_slug=payload.task_slug,
        custom_context=payload.custom_context,
        operating_mode=payload.operating_mode,
        offline_mode=payload.offline_mode,
        provider=payload.provider,
        model_id=payload.model_id,
        interrupt_before=payload.interrupt_before,
        mock_reply=payload.mock_reply,
        source="debug_ui",
    )
    return {
        "status": "ok",
        "run": run.to_detail_dict(),
        "final_reply": result.get("final_reply"),
    }


@router.get("/api/scenarios", dependencies=[Depends(_require_debug_enabled)])
def list_debug_scenarios() -> dict[str, Any]:
    """Return all built-in integration scenario presets."""
    return {
        "scenarios": [spec.model_dump(mode="json") for spec in BUILTIN_SCENARIOS.values()]
    }


@router.post("/api/scenarios/run", dependencies=[Depends(_require_debug_enabled)])
def run_debug_scenario(payload: DebugScenarioRunPayload) -> JSONResponse:
    """Execute a built-in or custom `ScenarioSpec` and return the assertion scorecard."""
    if payload.scenario is not None:
        spec = payload.scenario.model_copy(deep=True)
    elif payload.preset_id and payload.preset_id in BUILTIN_SCENARIOS:
        spec = BUILTIN_SCENARIOS[payload.preset_id].model_copy(deep=True)
    else:
        raise HTTPException(
            status_code=400,
            detail="Provide either a valid 'preset_id' or an inline 'scenario' spec.",
        )

    if payload.provider is not None:
        spec.provider = payload.provider
    if payload.model_id is not None:
        spec.model_id = payload.model_id
    if payload.session_id is not None:
        spec.session_id = payload.session_id
    if payload.offline_mode is not None:
        spec.offline_mode = payload.offline_mode

    result = run_scenario(spec)
    return JSONResponse(content=result)


@router.get("/api/ollama-logs", dependencies=[Depends(_require_debug_enabled)])
def get_debug_ollama_logs(lines: int = 120, high_signal_only: bool = False) -> dict[str, Any]:
    """Return recent `ollama serve` slot telemetry logs."""
    return get_recent_ollama_logs(limit=lines, high_signal_only=high_signal_only)


__all__ = [
    "is_debug_ui_enabled",
    "router",
]
