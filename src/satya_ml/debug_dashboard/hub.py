"""In-process LangGraph execution observer, SSE fan-out hub, breakpoint controller, and JSON trace store."""

from __future__ import annotations

import asyncio
from collections import deque
from collections.abc import Callable, Mapping, Sequence
from dataclasses import asdict, dataclass, field
from datetime import datetime, timezone
from enum import Enum
import json
import logging
import os
from pathlib import Path
import threading
import time
from typing import Any, Literal
import uuid

from langchain_core.messages import AIMessage, BaseMessage, HumanMessage, SystemMessage, ToolMessage
from pydantic import BaseModel

from satya_ml.ai_gm.ollama_runner import get_logs_dir
from satya_ml.ai_gm.telemetry import (
    ensure_turn_context,
    get_current_trace_id,
    redact_sensitive_text,
    turn_telemetry_scope,
)
from satya_ml.signaling_client import offline_signaling_scope

logger = logging.getLogger("satya_ml.debug_dashboard.hub")

AgentKind = Literal["ai_gm", "ai_player"]
RunStatus = Literal["running", "paused", "completed", "error", "aborted"]
NodeStatus = Literal["running", "completed", "error"]
RunSource = Literal["signaling", "debug_ui", "scenario"]

_MAX_STRING_CHARS = 32768
_DEFAULT_MAX_RUNS = 100
_DEFAULT_MAX_PERSISTED_TRACES = 200
_MAX_QUEUE_SIZE = 256


def _utc_now_iso() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.%f")[:-3] + "Z"


def _safe_filename_part(raw: str) -> str:
    cleaned = "".join(ch if ch.isalnum() or ch in ("-", "_") else "_" for ch in (raw or "default"))
    return (cleaned[:48] or "default").strip("_") or "default"


def _default_trace_dir() -> Path | None:
    explicit = os.environ.get("SATYA_AI_DEBUG_TRACE_DIR", "").strip()
    if explicit:
        p = Path(explicit)
        p.mkdir(parents=True, exist_ok=True)
        return p
    if os.environ.get("PYTEST_CURRENT_TEST"):
        return None
    trace_dir = get_logs_dir() / "debug_runs"
    trace_dir.mkdir(parents=True, exist_ok=True)
    return trace_dir


def _truncate_and_redact(text: str, *, max_chars: int = _MAX_STRING_CHARS) -> str:
    redacted = redact_sensitive_text(text)
    if len(redacted) <= max_chars:
        return redacted
    return redacted[: max_chars - 24] + f"... [+{len(redacted) - max_chars} chars]"


def serialize_message(msg: BaseMessage) -> dict[str, Any]:
    """Convert a LangChain `BaseMessage` into a redacted, JSON-safe dictionary."""
    if isinstance(msg, SystemMessage):
        role = "system"
    elif isinstance(msg, HumanMessage):
        role = "human"
    elif isinstance(msg, AIMessage):
        role = "ai"
    elif isinstance(msg, ToolMessage):
        role = "tool"
    else:
        role = str(getattr(msg, "type", "message") or "message").lower()

    raw_content = getattr(msg, "content", "")
    content_safe = to_json_safe(raw_content) if not isinstance(raw_content, str) else _truncate_and_redact(raw_content)

    entry: dict[str, Any] = {
        "role": role,
        "type": type(msg).__name__,
        "id": getattr(msg, "id", None),
        "name": getattr(msg, "name", None),
        "content": content_safe,
    }
    tool_calls = getattr(msg, "tool_calls", None)
    if tool_calls:
        entry["tool_calls"] = to_json_safe(tool_calls)
    tool_call_id = getattr(msg, "tool_call_id", None)
    if tool_call_id:
        entry["tool_call_id"] = str(tool_call_id)
    return entry


def to_json_safe(value: Any, *, _depth: int = 0) -> Any:
    """Recursively convert arbitrary Python / LangGraph objects into redacted JSON-safe structures."""
    if _depth > 8:
        return _truncate_and_redact(str(value), max_chars=512)
    if value is None or isinstance(value, (bool, int, float)):
        return value
    if isinstance(value, str):
        return _truncate_and_redact(value)
    if isinstance(value, BaseMessage):
        return serialize_message(value)
    if isinstance(value, Enum):
        return to_json_safe(value.value, _depth=_depth + 1)
    if isinstance(value, Path):
        return str(value)
    if isinstance(value, BaseModel):
        return to_json_safe(value.model_dump(mode="json"), _depth=_depth + 1)
    if isinstance(value, Mapping):
        return {
            str(k): to_json_safe(v, _depth=_depth + 1)
            for k, v in value.items()
        }
    if isinstance(value, (list, tuple, set, frozenset)):
        return [to_json_safe(item, _depth=_depth + 1) for item in value]
    if hasattr(value, "__dataclass_fields__"):
        return to_json_safe(asdict(value), _depth=_depth + 1)
    return _truncate_and_redact(str(value), max_chars=2048)


@dataclass
class NodeStepRecord:
    """Execution record for one node invocation inside a LangGraph turn."""

    step_index: int
    task_id: str
    node_name: str
    status: NodeStatus = "running"
    started_at_iso: str = field(default_factory=_utc_now_iso)
    ended_at_iso: str | None = None
    duration_ms: int = 0
    state_delta: dict[str, Any] = field(default_factory=dict)
    tool_names: list[str] = field(default_factory=list)
    error: str | None = None
    started_perf: float = field(default_factory=time.perf_counter, repr=False)

    def to_dict(self) -> dict[str, Any]:
        return {
            "step_index": self.step_index,
            "task_id": self.task_id,
            "node_name": self.node_name,
            "status": self.status,
            "started_at_iso": self.started_at_iso,
            "ended_at_iso": self.ended_at_iso,
            "duration_ms": self.duration_ms,
            "state_delta": self.state_delta,
            "tool_names": list(self.tool_names),
            "error": self.error,
        }


@dataclass
class TurnRunRecord:
    """Full observable record of a single AI GM or AI Player LangGraph turn."""

    run_id: str
    agent_kind: AgentKind
    session_id: str
    player_user_id: str | None = None
    thread_id: str = ""
    trace_id: str = ""
    source: str = "signaling"
    status: RunStatus = "running"
    started_at_iso: str = field(default_factory=_utc_now_iso)
    ended_at_iso: str | None = None
    duration_ms: int = 0
    provider: str = ""
    model_id: str = ""
    operating_mode: str = "autonomous"
    trigger_summary: str = ""
    final_channel: str | None = None
    final_reply: str | None = None
    party_gate_status: str | None = None
    loaded_tool_packs: list[str] = field(default_factory=list)
    loaded_tool_names: list[str] = field(default_factory=list)
    tool_invocations: list[dict[str, Any]] = field(default_factory=list)
    turn_logs: list[dict[str, Any]] = field(default_factory=list)
    messages: list[dict[str, Any]] = field(default_factory=list)
    steps: list[NodeStepRecord] = field(default_factory=list)
    next_nodes: list[str] = field(default_factory=list)
    interrupt_before: list[str] = field(default_factory=list)
    state_preview: dict[str, Any] = field(default_factory=dict)
    error: str | None = None
    persisted_path: str | None = None
    started_perf: float = field(default_factory=time.perf_counter, repr=False)

    def to_summary_dict(self) -> dict[str, Any]:
        return {
            "run_id": self.run_id,
            "agent_kind": self.agent_kind,
            "session_id": self.session_id,
            "player_user_id": self.player_user_id,
            "thread_id": self.thread_id,
            "trace_id": self.trace_id,
            "source": self.source,
            "status": self.status,
            "started_at_iso": self.started_at_iso,
            "ended_at_iso": self.ended_at_iso,
            "duration_ms": self.duration_ms,
            "provider": self.provider,
            "model_id": self.model_id,
            "operating_mode": self.operating_mode,
            "trigger_summary": self.trigger_summary,
            "final_channel": self.final_channel,
            "party_gate_status": self.party_gate_status,
            "step_count": len(self.steps),
            "tool_count": len(self.tool_invocations),
            "next_nodes": list(self.next_nodes),
            "interrupt_before": list(self.interrupt_before),
            "error": self.error,
        }

    def to_detail_dict(self) -> dict[str, Any]:
        detail = self.to_summary_dict()
        detail.update(
            {
                "final_reply": self.final_reply,
                "loaded_tool_packs": list(self.loaded_tool_packs),
                "loaded_tool_names": list(self.loaded_tool_names),
                "tool_invocations": list(self.tool_invocations),
                "turn_logs": list(self.turn_logs),
                "messages": list(self.messages),
                "steps": [step.to_dict() for step in self.steps],
                "state_preview": dict(self.state_preview),
                "persisted_path": self.persisted_path,
            }
        )
        return detail

    @classmethod
    def from_detail_dict(cls, data: Mapping[str, Any], *, persisted_path: str | None = None) -> TurnRunRecord:
        steps: list[NodeStepRecord] = []
        for idx, raw_step in enumerate(data.get("steps") or []):
            if not isinstance(raw_step, Mapping):
                continue
            steps.append(
                NodeStepRecord(
                    step_index=int(raw_step.get("step_index", idx)),
                    task_id=str(raw_step.get("task_id") or f"step-{idx}"),
                    node_name=str(raw_step.get("node_name") or ""),
                    status=str(raw_step.get("status") or "completed"),  # type: ignore[arg-type]
                    started_at_iso=str(raw_step.get("started_at_iso") or ""),
                    ended_at_iso=raw_step.get("ended_at_iso"),
                    duration_ms=int(raw_step.get("duration_ms") or 0),
                    state_delta=dict(raw_step.get("state_delta") or {}),
                    tool_names=list(raw_step.get("tool_names") or []),
                    error=raw_step.get("error"),
                )
            )
        return cls(
            run_id=str(data.get("run_id") or uuid.uuid4().hex[:12]),
            agent_kind="ai_player" if data.get("agent_kind") == "ai_player" else "ai_gm",
            session_id=str(data.get("session_id") or "default"),
            player_user_id=data.get("player_user_id"),
            thread_id=str(data.get("thread_id") or ""),
            trace_id=str(data.get("trace_id") or ""),
            source=str(data.get("source") or "persisted"),
            status=str(data.get("status") or "completed"),  # type: ignore[arg-type]
            started_at_iso=str(data.get("started_at_iso") or ""),
            ended_at_iso=data.get("ended_at_iso"),
            duration_ms=int(data.get("duration_ms") or 0),
            provider=str(data.get("provider") or ""),
            model_id=str(data.get("model_id") or ""),
            operating_mode=str(data.get("operating_mode") or "autonomous"),
            trigger_summary=str(data.get("trigger_summary") or ""),
            final_channel=data.get("final_channel"),
            final_reply=data.get("final_reply"),
            party_gate_status=data.get("party_gate_status"),
            loaded_tool_packs=list(data.get("loaded_tool_packs") or []),
            loaded_tool_names=list(data.get("loaded_tool_names") or []),
            tool_invocations=list(data.get("tool_invocations") or []),
            turn_logs=list(data.get("turn_logs") or []),
            messages=list(data.get("messages") or []),
            steps=steps,
            next_nodes=list(data.get("next_nodes") or []),
            interrupt_before=list(data.get("interrupt_before") or []),
            state_preview=dict(data.get("state_preview") or {}),
            error=data.get("error"),
            persisted_path=persisted_path or data.get("persisted_path"),
        )


@dataclass
class PausedRunContext:
    """In-memory handle allowing a paused `interrupt_before` run to be stepped, patched, or resumed."""

    run_id: str
    agent_kind: AgentKind
    session_id: str
    player_user_id: str | None
    thread_id: str
    graph: Any
    config: dict[str, Any]
    offline_mode: bool
    trace_id: str
    lock_getter: Callable[[], threading.Lock] | None = None


class _DummyTopologyModel:
    """Minimal chat-model stand-in used strictly to inspect compiled LangGraph topology."""

    def bind_tools(self, _tools: Any, **_kwargs: Any) -> _DummyTopologyModel:
        return self

    def invoke(self, _messages: Any, **_kwargs: Any) -> AIMessage:
        return AIMessage(content="")


_TOPOLOGY_LOCK = threading.Lock()
_CACHED_TOPOLOGY: dict[str, dict[str, Any]] | None = None


def _graph_to_topology_dict(compiled_graph: Any) -> dict[str, Any]:
    drawable = compiled_graph.get_graph()
    nodes: list[dict[str, Any]] = []
    for node_id in drawable.nodes:
        kind = "start" if node_id == "__start__" else ("end" if node_id == "__end__" else "node")
        label = "START" if kind == "start" else ("END" if kind == "end" else str(node_id))
        nodes.append({"id": str(node_id), "label": label, "kind": kind})

    edges: list[dict[str, Any]] = []
    for edge in drawable.edges:
        edges.append(
            {
                "source": str(edge.source),
                "target": str(edge.target),
                "conditional": bool(getattr(edge, "conditional", False)),
                "data": str(edge.data) if getattr(edge, "data", None) is not None else None,
            }
        )
    return {"nodes": nodes, "edges": edges}


def extract_graph_topology(*, force_refresh: bool = False) -> dict[str, dict[str, Any]]:
    """Return canonical node/edge topology extracted directly from both compiled LangGraphs."""
    global _CACHED_TOPOLOGY
    with _TOPOLOGY_LOCK:
        if _CACHED_TOPOLOGY is not None and not force_refresh:
            return _CACHED_TOPOLOGY

        from satya_ml.ai_gm.graph import build_ai_gm_graph
        from satya_ml.ai_gm.models import SessionLlmConfig
        from satya_ml.ai_player.graph import build_ai_player_graph

        dummy = _DummyTopologyModel()
        cfg = SessionLlmConfig(provider="ollama", model_id="topology-inspector")
        gm_graph = build_ai_gm_graph(cfg, chat_model=dummy, lazy_tool_loading=True)
        player_graph = build_ai_player_graph(cfg, chat_model=dummy)
        _CACHED_TOPOLOGY = {
            "ai_gm": _graph_to_topology_dict(gm_graph),
            "ai_player": _graph_to_topology_dict(player_graph),
        }
        return _CACHED_TOPOLOGY


def get_agent_node_names(agent_kind: AgentKind) -> list[str]:
    """Return all executable domain node names for `agent_kind` (excluding `__start__` and `__end__`)."""
    topo = extract_graph_topology().get(agent_kind) or {}
    return [
        str(n["id"])
        for n in (topo.get("nodes") or [])
        if n.get("kind") == "node"
    ]


class GraphExecutionHub:
    """Thread-safe ring buffer, SSE subscriber registry, paused-run registry, and JSON trace store."""

    def __init__(
        self,
        *,
        max_runs: int = _DEFAULT_MAX_RUNS,
        trace_dir: Path | None = None,
        max_persisted_traces: int | None = None,
        auto_load_traces: bool = True,
    ) -> None:
        self._lock = threading.RLock()
        self._max_runs = max(10, int(max_runs))
        self._runs_order: deque[str] = deque()
        self._runs_by_id: dict[str, TurnRunRecord] = {}
        self._paused_contexts: dict[str, PausedRunContext] = {}
        self._subscribers: dict[int, tuple[asyncio.AbstractEventLoop, asyncio.Queue[dict[str, Any]]]] = {}
        self._sub_seq = 0
        self._trace_dir = trace_dir if trace_dir is not None else _default_trace_dir()
        env_max = os.environ.get("SATYA_AI_DEBUG_MAX_TRACES", "").strip()
        resolved_cap = (
            max_persisted_traces
            if max_persisted_traces is not None
            else (int(env_max) if env_max.isdigit() else _DEFAULT_MAX_PERSISTED_TRACES)
        )
        self._max_persisted_traces = max(1, resolved_cap)
        if auto_load_traces and self._trace_dir is not None:
            self.load_persisted_traces()

    @property
    def trace_dir(self) -> Path | None:
        return self._trace_dir

    def set_trace_dir(self, trace_dir: Path | None, *, reload: bool = True) -> None:
        with self._lock:
            self._trace_dir = trace_dir
            if trace_dir is not None:
                trace_dir.mkdir(parents=True, exist_ok=True)
                if reload:
                    self.load_persisted_traces()

    def register_run(self, run: TurnRunRecord) -> None:
        with self._lock:
            if run.run_id in self._runs_by_id:
                self._runs_by_id[run.run_id] = run
                return
            self._runs_by_id[run.run_id] = run
            self._runs_order.append(run.run_id)
            while len(self._runs_order) > self._max_runs:
                evicted_id = self._runs_order.popleft()
                self._runs_by_id.pop(evicted_id, None)
                self._paused_contexts.pop(evicted_id, None)

    def get_run(self, run_id: str) -> TurnRunRecord | None:
        with self._lock:
            return self._runs_by_id.get(run_id)

    def list_runs(
        self,
        *,
        session_id: str | None = None,
        agent_kind: str | None = None,
        limit: int = 50,
    ) -> list[dict[str, Any]]:
        with self._lock:
            items: list[dict[str, Any]] = []
            for run_id in reversed(self._runs_order):
                run = self._runs_by_id.get(run_id)
                if run is None:
                    continue
                if session_id and run.session_id != session_id:
                    continue
                if agent_kind and run.agent_kind != agent_kind:
                    continue
                items.append(run.to_summary_dict())
                if len(items) >= max(1, limit):
                    break
            return items

    def list_known_sessions(self) -> list[str]:
        with self._lock:
            seen: dict[str, None] = {}
            for run_id in reversed(self._runs_order):
                run = self._runs_by_id.get(run_id)
                if run is not None and run.session_id:
                    seen.setdefault(run.session_id, None)
            return list(seen.keys())

    def set_paused_context(self, ctx: PausedRunContext) -> None:
        with self._lock:
            self._paused_contexts[ctx.run_id] = ctx

    def get_paused_context(self, run_id: str) -> PausedRunContext | None:
        with self._lock:
            return self._paused_contexts.get(run_id)

    def pop_paused_context(self, run_id: str) -> PausedRunContext | None:
        with self._lock:
            return self._paused_contexts.pop(run_id, None)

    def subscribe(self) -> tuple[int, asyncio.Queue[dict[str, Any]]]:
        loop = asyncio.get_running_loop()
        queue: asyncio.Queue[dict[str, Any]] = asyncio.Queue(maxsize=_MAX_QUEUE_SIZE)
        with self._lock:
            self._sub_seq += 1
            sub_id = self._sub_seq
            self._subscribers[sub_id] = (loop, queue)
        return sub_id, queue

    def unsubscribe(self, sub_id: int) -> None:
        with self._lock:
            self._subscribers.pop(sub_id, None)

    def publish_event(self, event_type: str, data: dict[str, Any]) -> None:
        event = {
            "type": event_type,
            "ts": _utc_now_iso(),
            "data": to_json_safe(data),
        }
        with self._lock:
            subscribers = list(self._subscribers.items())

        def _enqueue(q: asyncio.Queue[dict[str, Any]], item: dict[str, Any]) -> None:
            if q.full():
                try:
                    q.get_nowait()
                except asyncio.QueueEmpty:
                    pass
            try:
                q.put_nowait(item)
            except asyncio.QueueFull:
                pass

        for sub_id, (loop, queue) in subscribers:
            if loop.is_closed():
                self.unsubscribe(sub_id)
                continue
            try:
                loop.call_soon_threadsafe(_enqueue, queue, event)
            except RuntimeError:
                self.unsubscribe(sub_id)

    def persist_run_trace(self, run: TurnRunRecord) -> str | None:
        with self._lock:
            trace_dir = self._trace_dir
            max_files = self._max_persisted_traces
        if trace_dir is None:
            return None
        try:
            trace_dir.mkdir(parents=True, exist_ok=True)
            ts_slug = (run.started_at_iso or _utc_now_iso()).replace(":", "-").replace(".", "-")
            sess_slug = _safe_filename_part(run.session_id)
            file_name = f"{ts_slug}_{run.agent_kind}_{sess_slug}_{run.run_id}.json"
            target = trace_dir / file_name
            tmp_target = trace_dir / f".{file_name}.tmp"
            payload = run.to_detail_dict()
            payload["persisted_path"] = str(target)
            tmp_target.write_text(json.dumps(payload, indent=2), encoding="utf-8", newline="\n")
            os.replace(tmp_target, target)
            run.persisted_path = str(target)
            self._prune_trace_files(trace_dir, max_files)
            return str(target)
        except OSError as exc:
            logger.debug("Failed to persist debug trace %s: %s", run.run_id, exc)
            return None

    @staticmethod
    def _prune_trace_files(trace_dir: Path, max_files: int) -> None:
        try:
            files = sorted(trace_dir.glob("*.json"), key=lambda p: p.name)
            excess = len(files) - max_files
            for stale in files[: max(0, excess)]:
                stale.unlink(missing_ok=True)
        except OSError:
            pass

    def load_persisted_traces(self, limit: int | None = None) -> int:
        with self._lock:
            trace_dir = self._trace_dir
            cap = min(self._max_runs, limit or self._max_runs)
        if trace_dir is None or not trace_dir.exists():
            return 0
        loaded = 0
        try:
            files = sorted(trace_dir.glob("*.json"), key=lambda p: p.name)[-cap:]
        except OSError:
            return 0
        for path in files:
            try:
                raw = json.loads(path.read_text(encoding="utf-8"))
                if not isinstance(raw, dict):
                    continue
                run = TurnRunRecord.from_detail_dict(raw, persisted_path=str(path))
                if self.get_run(run.run_id) is None:
                    self.register_run(run)
                    loaded += 1
            except Exception:
                continue
        return loaded

    def clear_traces(self, *, delete_files: bool = False) -> int:
        with self._lock:
            cleared = len(self._runs_by_id)
            self._runs_order.clear()
            self._runs_by_id.clear()
            self._paused_contexts.clear()
            trace_dir = self._trace_dir
        if delete_files and trace_dir is not None and trace_dir.exists():
            for path in trace_dir.glob("*.json"):
                try:
                    path.unlink(missing_ok=True)
                except OSError:
                    pass
        self.publish_event("traces_cleared", {"delete_files": delete_files})
        return cleared


GRAPH_HUB = GraphExecutionHub()


def _extract_state_preview(state: Mapping[str, Any]) -> dict[str, Any]:
    """Return a concise redacted snapshot of key LangGraph state fields for quick inspection."""
    keys = (
        "session_id",
        "operating_mode",
        "combat_active",
        "party_gate_status",
        "awaiting_user_ids",
        "awaiting_display_names",
        "active_director_mandate",
        "loaded_tool_packs",
        "loaded_tool_names",
        "final_channel",
        "trace_id",
    )
    preview: dict[str, Any] = {}
    for key in keys:
        if key in state:
            preview[key] = to_json_safe(state[key])
    if isinstance(state.get("npc_turn_summaries"), list):
        preview["npc_turn_summaries"] = to_json_safe(state["npc_turn_summaries"])
    if isinstance(state.get("roll_results"), list):
        preview["roll_results"] = to_json_safe(state["roll_results"])
    if isinstance(state.get("active_npcs"), Mapping):
        preview["active_npc_count"] = len(state["active_npcs"])
    if isinstance(state.get("active_map"), Mapping):
        active_map = state["active_map"]
        preview["active_map_summary"] = {
            "map_id": active_map.get("map_id"),
            "title": active_map.get("title") or active_map.get("theme"),
            "token_count": len(active_map.get("tokens") or []),
        }
    return preview


def _extract_tool_names_from_delta(delta: Mapping[str, Any]) -> list[str]:
    names: list[str] = []
    for msg in delta.get("messages") or []:
        for tc in getattr(msg, "tool_calls", None) or []:
            t_name = str(tc.get("name") or "").strip()
            if t_name and t_name not in names:
                names.append(t_name)
        if isinstance(msg, ToolMessage):
            t_name = str(getattr(msg, "name", "") or "").strip()
            if t_name and t_name not in names:
                names.append(t_name)
    for inv in delta.get("tool_invocations") or []:
        if isinstance(inv, Mapping):
            t_name = str(inv.get("tool_name") or "").strip()
            if t_name and t_name not in names:
                names.append(t_name)
    return names


def _populate_run_from_final_state(run: TurnRunRecord, final_state: Mapping[str, Any]) -> None:
    if not final_state:
        return
    if final_state.get("final_reply") is not None:
        run.final_reply = _truncate_and_redact(str(final_state.get("final_reply") or ""))
    if final_state.get("final_channel") is not None:
        run.final_channel = str(final_state.get("final_channel"))
    if final_state.get("party_gate_status") is not None:
        run.party_gate_status = str(final_state.get("party_gate_status"))
    if final_state.get("trace_id"):
        run.trace_id = str(final_state.get("trace_id"))
    if isinstance(final_state.get("loaded_tool_packs"), list):
        run.loaded_tool_packs = [str(p) for p in final_state["loaded_tool_packs"]]
    if isinstance(final_state.get("loaded_tool_names"), list):
        run.loaded_tool_names = [str(n) for n in final_state["loaded_tool_names"]]
    if isinstance(final_state.get("tool_invocations"), list):
        run.tool_invocations = to_json_safe(final_state["tool_invocations"])
    if isinstance(final_state.get("turn_logs"), list):
        run.turn_logs = to_json_safe(final_state["turn_logs"])
    if isinstance(final_state.get("messages"), list):
        run.messages = [to_json_safe(m) for m in final_state["messages"][-40:]]
    run.state_preview = _extract_state_preview(final_state)


def _find_or_create_step(
    run: TurnRunRecord,
    *,
    task_id: str | None,
    node_name: str,
) -> NodeStepRecord:
    if task_id:
        for step in reversed(run.steps):
            if step.task_id == task_id:
                return step
    for step in reversed(run.steps):
        if step.node_name == node_name and step.status == "running":
            return step
    step = NodeStepRecord(
        step_index=len(run.steps),
        task_id=task_id or f"task-{len(run.steps)}",
        node_name=node_name,
    )
    run.steps.append(step)
    return step


def run_observed_graph(
    graph: Any,
    initial_state: dict[str, Any] | None,
    config: dict[str, Any],
    *,
    agent_kind: AgentKind,
    session_id: str,
    player_user_id: str | None = None,
    trace_id: str = "",
    provider: str = "",
    model_id: str = "",
    operating_mode: str = "autonomous",
    trigger_summary: str = "",
    source: str = "signaling",
    interrupt_before: Sequence[str] | None = None,
    offline_mode: bool = False,
    hub: GraphExecutionHub | None = None,
    existing_run: TurnRunRecord | None = None,
    lock_getter: Callable[[], threading.Lock] | None = None,
) -> tuple[dict[str, Any], TurnRunRecord]:
    """Execute a compiled LangGraph with live node progress, delta capture, breakpoint pause, and trace output."""
    eff_hub = hub or GRAPH_HUB
    thread_id = str((config.get("configurable") or {}).get("thread_id") or session_id)
    eff_interrupts = [str(n).strip() for n in (interrupt_before or ()) if str(n).strip()]
    eff_trace_id = trace_id or get_current_trace_id() or ""

    if existing_run is not None:
        run = existing_run
        run.status = "running"
        run.next_nodes = []
        run.interrupt_before = eff_interrupts
        run.error = None
        if eff_trace_id:
            run.trace_id = eff_trace_id
    else:
        run = TurnRunRecord(
            run_id=uuid.uuid4().hex[:12],
            agent_kind=agent_kind,
            session_id=session_id,
            player_user_id=player_user_id,
            thread_id=thread_id,
            trace_id=eff_trace_id,
            source=source,
            status="running",
            provider=provider,
            model_id=model_id,
            operating_mode=operating_mode,
            trigger_summary=_truncate_and_redact(trigger_summary, max_chars=240),
            interrupt_before=eff_interrupts,
        )
        eff_hub.register_run(run)

    eff_hub.publish_event("turn_started", {"run": run.to_summary_dict()})

    final_state: dict[str, Any] = {}
    try:
        if not hasattr(graph, "stream"):
            raw_res = graph.invoke(initial_state, config=config)
            final_state = dict(raw_res) if isinstance(raw_res, Mapping) else {}
        else:
            stream_kwargs: dict[str, Any] = {
                "config": config,
                "stream_mode": ["tasks", "updates", "values"],
            }
            if eff_interrupts:
                stream_kwargs["interrupt_before"] = eff_interrupts

            for item in graph.stream(initial_state, **stream_kwargs):
                if not (isinstance(item, tuple) and len(item) == 2):
                    continue
                mode, chunk = item
                if mode == "tasks" and isinstance(chunk, Mapping):
                    t_id = str(chunk.get("id") or "")
                    n_name = str(chunk.get("name") or "")
                    if not n_name:
                        continue
                    is_start = "input" in chunk and "result" not in chunk and "error" not in chunk
                    if is_start:
                        step = NodeStepRecord(
                            step_index=len(run.steps),
                            task_id=t_id or f"task-{len(run.steps)}",
                            node_name=n_name,
                            status="running",
                        )
                        run.steps.append(step)
                        eff_hub.publish_event(
                            "node_started",
                            {"run_id": run.run_id, "agent_kind": agent_kind, "step": step.to_dict()},
                        )
                    else:
                        step = _find_or_create_step(run, task_id=t_id, node_name=n_name)
                        step.ended_at_iso = _utc_now_iso()
                        step.duration_ms = int(max(0.0, (time.perf_counter() - step.started_perf) * 1000.0))
                        if chunk.get("error") is not None:
                            step.status = "error"
                            step.error = _truncate_and_redact(str(chunk.get("error")), max_chars=1024)
                            eff_hub.publish_event(
                                "node_error",
                                {"run_id": run.run_id, "agent_kind": agent_kind, "step": step.to_dict()},
                            )
                        else:
                            step.status = "completed"
                            if isinstance(chunk.get("result"), Mapping) and not step.state_delta:
                                step.state_delta = to_json_safe(chunk["result"])
                                step.tool_names = _extract_tool_names_from_delta(chunk["result"])
                            eff_hub.publish_event(
                                "node_completed",
                                {"run_id": run.run_id, "agent_kind": agent_kind, "step": step.to_dict()},
                            )
                elif mode == "updates" and isinstance(chunk, Mapping):
                    for n_name, delta in chunk.items():
                        if str(n_name).startswith("__"):
                            continue
                        if isinstance(delta, Mapping):
                            target_step = None
                            for cand in reversed(run.steps):
                                if cand.node_name == str(n_name):
                                    target_step = cand
                                    break
                            if target_step is None:
                                target_step = _find_or_create_step(
                                    run, task_id=None, node_name=str(n_name)
                                )
                                target_step.status = "completed"
                                target_step.ended_at_iso = _utc_now_iso()
                            target_step.state_delta = to_json_safe(delta)
                            target_step.tool_names = _extract_tool_names_from_delta(delta)
                            eff_hub.publish_event(
                                "node_delta",
                                {
                                    "run_id": run.run_id,
                                    "agent_kind": agent_kind,
                                    "step": target_step.to_dict(),
                                },
                            )
                elif mode == "values" and isinstance(chunk, Mapping):
                    final_state = dict(chunk)

        state_snap = graph.get_state(config) if hasattr(graph, "get_state") else None
        snap_values = getattr(state_snap, "values", None)
        if not final_state and isinstance(snap_values, Mapping):
            final_state = dict(snap_values)

        _populate_run_from_final_state(run, final_state)
        run.duration_ms = int(max(1.0, (time.perf_counter() - run.started_perf) * 1000.0))

        next_nodes = [str(n) for n in (getattr(state_snap, "next", None) or ())]
        if next_nodes:
            run.status = "paused"
            run.next_nodes = next_nodes
            eff_hub.set_paused_context(
                PausedRunContext(
                    run_id=run.run_id,
                    agent_kind=agent_kind,
                    session_id=session_id,
                    player_user_id=player_user_id,
                    thread_id=thread_id,
                    graph=graph,
                    config=config,
                    offline_mode=offline_mode,
                    trace_id=run.trace_id,
                    lock_getter=lock_getter,
                )
            )
            eff_hub.publish_event("turn_paused", {"run": run.to_summary_dict()})
            return final_state, run

        run.status = "completed"
        run.ended_at_iso = _utc_now_iso()
        run.next_nodes = []
        eff_hub.pop_paused_context(run.run_id)
        eff_hub.persist_run_trace(run)
        eff_hub.publish_event("turn_completed", {"run": run.to_summary_dict()})
        return final_state, run

    except Exception as exc:
        run.status = "error"
        run.ended_at_iso = _utc_now_iso()
        run.duration_ms = int(max(1.0, (time.perf_counter() - run.started_perf) * 1000.0))
        run.error = _truncate_and_redact(str(exc), max_chars=2048)
        eff_hub.pop_paused_context(run.run_id)
        eff_hub.persist_run_trace(run)
        eff_hub.publish_event("turn_completed", {"run": run.to_summary_dict()})
        raise


def resume_observed_run(
    run_id: str,
    *,
    step_only: bool = False,
    interrupt_before: Sequence[str] | None = None,
    state_patch: dict[str, Any] | None = None,
    hub: GraphExecutionHub | None = None,
) -> tuple[dict[str, Any], TurnRunRecord]:
    """Resume (or single-step) a paused `interrupt_before` run from its `MemorySaver` checkpoint."""
    eff_hub = hub or GRAPH_HUB
    ctx = eff_hub.get_paused_context(run_id)
    run = eff_hub.get_run(run_id)
    if ctx is None or run is None:
        raise KeyError(f"Run '{run_id}' is not paused or no longer exists")

    next_interrupts: list[str]
    if step_only:
        next_interrupts = get_agent_node_names(ctx.agent_kind)
    elif interrupt_before is not None:
        next_interrupts = [str(n).strip() for n in interrupt_before if str(n).strip()]
    else:
        next_interrupts = []

    def _run_under_scopes() -> tuple[dict[str, Any], TurnRunRecord]:
        with offline_signaling_scope(ctx.offline_mode):
            with turn_telemetry_scope(ctx.session_id, trace_id=ctx.trace_id or None) as telemetry:
                ensure_turn_context(ctx.session_id, trace_id=telemetry.trace_id)
                if state_patch and hasattr(ctx.graph, "update_state"):
                    ctx.graph.update_state(ctx.config, dict(state_patch))
                return run_observed_graph(
                    ctx.graph,
                    None,
                    ctx.config,
                    agent_kind=ctx.agent_kind,
                    session_id=ctx.session_id,
                    player_user_id=ctx.player_user_id,
                    trace_id=telemetry.trace_id,
                    provider=run.provider,
                    model_id=run.model_id,
                    operating_mode=run.operating_mode,
                    trigger_summary=run.trigger_summary,
                    source=run.source,
                    interrupt_before=next_interrupts,
                    offline_mode=ctx.offline_mode,
                    hub=eff_hub,
                    existing_run=run,
                    lock_getter=ctx.lock_getter,
                )

    if ctx.lock_getter is not None:
        with ctx.lock_getter():
            return _run_under_scopes()
    return _run_under_scopes()


def abort_paused_run(run_id: str, *, hub: GraphExecutionHub | None = None) -> TurnRunRecord:
    """Abort a paused run, persist its partial trace, and notify SSE subscribers."""
    eff_hub = hub or GRAPH_HUB
    run = eff_hub.get_run(run_id)
    ctx = eff_hub.pop_paused_context(run_id)
    if run is None or ctx is None:
        raise KeyError(f"Run '{run_id}' is not paused or no longer exists")
    run.status = "aborted"
    run.ended_at_iso = _utc_now_iso()
    run.duration_ms = int(max(1.0, (time.perf_counter() - run.started_perf) * 1000.0))
    run.next_nodes = []
    eff_hub.persist_run_trace(run)
    eff_hub.publish_event("turn_completed", {"run": run.to_summary_dict()})
    return run


__all__ = [
    "AgentKind",
    "GRAPH_HUB",
    "GraphExecutionHub",
    "NodeStatus",
    "NodeStepRecord",
    "PausedRunContext",
    "RunSource",
    "RunStatus",
    "TurnRunRecord",
    "abort_paused_run",
    "extract_graph_topology",
    "get_agent_node_names",
    "resume_observed_run",
    "run_observed_graph",
    "serialize_message",
    "to_json_safe",
]
