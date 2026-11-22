"""W3C `traceparent` propagation, `contextvars` call-graph & data-lineage threading,
bracketed plain-text log formatting, credential redaction, and bounded turn-scoped
`SessionLogEvent` buffering for `satya_ml.ai_gm`.

Implements Section 3.4, Section 4, and Section 6.1 of
`docs/technical/observability-and-avro-telemetry-design.md`.
"""

from __future__ import annotations

from collections.abc import Iterator, Mapping, Sequence
from contextlib import contextmanager
from contextvars import ContextVar
from dataclasses import dataclass, field
from datetime import datetime, timezone
import logging
import re
from typing import Any
import uuid

from satya_py.enums import AiGmOperatingMode, AiGmTurnStatus, LogLevel, McpServerTier

MAX_TURN_LOGS_DEFAULT = 32
MAX_LOG_LINE_CHARS_DEFAULT = 512

_TRACEPARENT_RE = re.compile(
    r"^\s*00-([0-9a-fA-F]{32})-([0-9a-fA-F]{16})-([0-9a-fA-F]{2})\s*$"
)

# Matches JSON/query/header secret assignments and Bearer tokens (Section 6.1)
_BEARER_RE = re.compile(r"(?i)\b(bearer\s+)[A-Za-z0-9._\-+/=]+")
_KV_SECRET_RE = re.compile(
    r"""(?i)(["']?(?:api[_-]?key|service[_-]?token|access[_-]?token|secret|password|authorization|x-satya-provider-key)["']?\s*[:=]\s*["']?)([^"',\s\}]+)"""
)

_SESSION_ID_VAR: ContextVar[str | None] = ContextVar("satya_session_id", default=None)
_TRACE_ID_VAR: ContextVar[str | None] = ContextVar("satya_trace_id", default=None)
_SPAN_ID_VAR: ContextVar[str | None] = ContextVar("satya_span_id", default=None)
_PARENT_SPAN_ID_VAR: ContextVar[str | None] = ContextVar("satya_parent_span_id", default=None)
_LINEAGE_ID_VAR: ContextVar[str | None] = ContextVar("satya_lineage_id", default=None)
_PARENT_LINEAGE_IDS_VAR: ContextVar[tuple[str, ...]] = ContextVar(
    "satya_parent_lineage_ids", default=()
)
_TURN_LOG_BUFFER_VAR: ContextVar[list[dict[str, Any]] | None] = ContextVar(
    "satya_turn_log_buffer", default=None
)
_EMITTED_EVENT_IDS_VAR: ContextVar[list[str] | None] = ContextVar(
    "satya_emitted_event_ids", default=None
)


def new_trace_id() -> str:
    """Return a 32-character lowercase hex W3C `trace_id`."""
    return uuid.uuid4().hex


def new_span_id() -> str:
    """Return a 16-character lowercase hex W3C `span_id`."""
    return uuid.uuid4().hex[:16]


def parse_traceparent(header: str | None) -> tuple[str | None, str | None]:
    """Parse a W3C `traceparent` header (`00-<trace_id>-<parent_span_id>-<flags>`).

    Returns `(trace_id, parent_span_id)` in lowercase hex, or `(None, None)` if invalid.
    """
    if not header or not isinstance(header, str):
        return None, None
    m = _TRACEPARENT_RE.match(header)
    if not m:
        return None, None
    trace_id = m.group(1).lower()
    span_id = m.group(2).lower()
    if trace_id == "0" * 32 or span_id == "0" * 16:
        return None, None
    return trace_id, span_id


def format_traceparent(trace_id: str, span_id: str, sampled: bool = True) -> str:
    """Format a W3C `traceparent` header string."""
    flags = "01" if sampled else "00"
    return f"00-{trace_id.lower()}-{span_id.lower()}-{flags}"


def redact_sensitive_text(text: str) -> str:
    """Scrub API keys, Bearer tokens, and authorization secrets from log lines and JSON strings."""
    if not text:
        return text
    scrubbed = _BEARER_RE.sub(r"\1[REDACTED]", text)
    scrubbed = _KV_SECRET_RE.sub(r"\1[REDACTED]", scrubbed)
    return scrubbed


def get_current_trace_id() -> str | None:
    """Return the active 32-hex W3C `trace_id` in the current context, if any."""
    return _TRACE_ID_VAR.get()


def get_current_span_id() -> str | None:
    """Return the active 16-hex W3C `span_id` in the current context, if any."""
    return _SPAN_ID_VAR.get()


def get_current_parent_span_id() -> str | None:
    """Return the caller 16-hex `parent_span_id` in the current context, if any."""
    return _PARENT_SPAN_ID_VAR.get()


def get_current_lineage_id() -> str | None:
    """Return the active data artifact `lineage_id` in the current context, if any."""
    return _LINEAGE_ID_VAR.get()


def get_current_parent_lineage_ids() -> list[str]:
    """Return the upstream `parent_lineage_ids` in the current context."""
    return list(_PARENT_LINEAGE_IDS_VAR.get())


def get_current_turn_logs() -> list[dict[str, Any]]:
    """Return a snapshot of buffered `GmSessionLogEntry` dicts for the current turn."""
    buf = _TURN_LOG_BUFFER_VAR.get()
    return list(buf) if buf is not None else []


def get_current_emitted_event_ids() -> list[str]:
    """Return all domain `event_id`s emitted during the current turn."""
    emitted = _EMITTED_EVENT_IDS_VAR.get()
    return list(emitted) if emitted is not None else []


def ensure_turn_context(
    session_id: str,
    *,
    trace_id: str | None = None,
    parent_span_id: str | None = None,
    lineage_id: str | None = None,
) -> str:
    """Ensure active `contextvars` telemetry state exists (when `graph.invoke` is called directly)."""
    if _SESSION_ID_VAR.get() != session_id:
        _SESSION_ID_VAR.set(session_id)
    cur_tid = _TRACE_ID_VAR.get()
    if not cur_tid:
        cur_tid = (trace_id or new_trace_id()).lower()
        if len(cur_tid) != 32:
            cur_tid = new_trace_id()
        _TRACE_ID_VAR.set(cur_tid)
        _SPAN_ID_VAR.set(new_span_id())
        _PARENT_SPAN_ID_VAR.set(parent_span_id)
    if lineage_id and not _LINEAGE_ID_VAR.get():
        _LINEAGE_ID_VAR.set(lineage_id)
    if _TURN_LOG_BUFFER_VAR.get() is None:
        _TURN_LOG_BUFFER_VAR.set([])
    if _EMITTED_EVENT_IDS_VAR.get() is None:
        _EMITTED_EVENT_IDS_VAR.set([])
    return cur_tid


def current_trace_headers() -> dict[str, str]:
    """Return outbound W3C `traceparent` and `X-Satya-Lineage-Id` headers for callbacks to Rust."""
    headers: dict[str, str] = {}
    tid = _TRACE_ID_VAR.get()
    sid = _SPAN_ID_VAR.get()
    if tid and sid:
        headers["traceparent"] = format_traceparent(tid, sid)
    lid = _LINEAGE_ID_VAR.get()
    if lid:
        headers["X-Satya-Lineage-Id"] = lid
    return headers


def record_emitted_event_id(event_id: str | None) -> None:
    """Record a domain `event_id` emitted during the current turn and advance data lineage."""
    if not event_id:
        return
    clean_id = str(event_id).strip()
    if not clean_id:
        return
    emitted = _EMITTED_EVENT_IDS_VAR.get()
    if emitted is not None and clean_id not in emitted:
        emitted.append(clean_id)
    prev_lineage = _LINEAGE_ID_VAR.get()
    if prev_lineage and prev_lineage != clean_id:
        existing_parents = list(_PARENT_LINEAGE_IDS_VAR.get())
        if prev_lineage not in existing_parents:
            existing_parents.append(prev_lineage)
        _PARENT_LINEAGE_IDS_VAR.set(tuple(existing_parents))
    _LINEAGE_ID_VAR.set(clean_id)


@dataclass
class TurnTelemetryHandle:
    """Active telemetry handle for a single LangGraph AI GM turn."""

    session_id: str
    trace_id: str
    root_span_id: str
    parent_span_id: str | None
    initial_lineage_id: str | None
    log_buffer: list[dict[str, Any]] = field(default_factory=list)
    emitted_event_ids: list[str] = field(default_factory=list)

    def drain_turn_logs(self) -> list[dict[str, Any]]:
        """Return a snapshot of all buffered `INFO`/`WARN`/`ERROR` `GmSessionLogEntry` dicts."""
        return list(self.log_buffer)


@contextmanager
def turn_telemetry_scope(
    session_id: str,
    *,
    traceparent: str | None = None,
    trace_id: str | None = None,
    parent_span_id: str | None = None,
    lineage_id: str | None = None,
) -> Iterator[TurnTelemetryHandle]:
    """Establish the root `contextvars` telemetry scope and bounded log buffer for an AI GM turn."""
    parsed_tid, parsed_psid = parse_traceparent(traceparent)
    eff_tid = (parsed_tid or trace_id or new_trace_id()).lower()
    if len(eff_tid) != 32:
        eff_tid = new_trace_id()
    eff_psid = parsed_psid or parent_span_id
    root_sid = new_span_id()

    handle = TurnTelemetryHandle(
        session_id=session_id,
        trace_id=eff_tid,
        root_span_id=root_sid,
        parent_span_id=eff_psid,
        initial_lineage_id=lineage_id,
    )

    tok_sess = _SESSION_ID_VAR.set(session_id)
    tok_tid = _TRACE_ID_VAR.set(eff_tid)
    tok_sid = _SPAN_ID_VAR.set(root_sid)
    tok_psid = _PARENT_SPAN_ID_VAR.set(eff_psid)
    tok_lid = _LINEAGE_ID_VAR.set(lineage_id)
    tok_plids = _PARENT_LINEAGE_IDS_VAR.set(())
    tok_buf = _TURN_LOG_BUFFER_VAR.set(handle.log_buffer)
    tok_emit = _EMITTED_EVENT_IDS_VAR.set(handle.emitted_event_ids)
    try:
        yield handle
    finally:
        _SESSION_ID_VAR.reset(tok_sess)
        _TRACE_ID_VAR.reset(tok_tid)
        _SPAN_ID_VAR.reset(tok_sid)
        _PARENT_SPAN_ID_VAR.reset(tok_psid)
        _LINEAGE_ID_VAR.reset(tok_lid)
        _PARENT_LINEAGE_IDS_VAR.reset(tok_plids)
        _TURN_LOG_BUFFER_VAR.reset(tok_buf)
        _EMITTED_EVENT_IDS_VAR.reset(tok_emit)


@contextmanager
def span_scope(
    _span_name: str,
    *,
    new_lineage_id: str | None = None,
    extra_parent_lineages: list[str] | None = None,
) -> Iterator[str]:
    """Push a child `span_id` (`child_span_id <- current_span_id`) and optional `lineage_id` edge."""
    parent_sid = _SPAN_ID_VAR.get()
    child_sid = new_span_id()
    tok_psid = _PARENT_SPAN_ID_VAR.set(parent_sid)
    tok_sid = _SPAN_ID_VAR.set(child_sid)

    tok_lid = None
    tok_plids = None
    if new_lineage_id is not None or extra_parent_lineages:
        prev_lid = _LINEAGE_ID_VAR.get()
        parents: list[str] = []
        if prev_lid:
            parents.append(prev_lid)
        for p in extra_parent_lineages or []:
            if p and p not in parents:
                parents.append(p)
        tok_plids = _PARENT_LINEAGE_IDS_VAR.set(tuple(parents))
        if new_lineage_id is not None:
            tok_lid = _LINEAGE_ID_VAR.set(new_lineage_id)

    try:
        yield child_sid
    finally:
        _SPAN_ID_VAR.reset(tok_sid)
        _PARENT_SPAN_ID_VAR.reset(tok_psid)
        if tok_lid is not None:
            _LINEAGE_ID_VAR.reset(tok_lid)
        if tok_plids is not None:
            _PARENT_LINEAGE_IDS_VAR.reset(tok_plids)


_PYTHON_TO_AVRO_LOG_LEVEL: dict[str, str] = {
    "WARNING": LogLevel.WARN.value,
    "CRITICAL": LogLevel.FATAL.value,
    LogLevel.TRACE.value: LogLevel.TRACE.value,
    LogLevel.DEBUG.value: LogLevel.DEBUG.value,
    LogLevel.INFO.value: LogLevel.INFO.value,
    LogLevel.WARN.value: LogLevel.WARN.value,
    LogLevel.ERROR.value: LogLevel.ERROR.value,
    LogLevel.FATAL.value: LogLevel.FATAL.value,
}


def _map_python_level_to_avro(levelname: str) -> str:
    return _PYTHON_TO_AVRO_LOG_LEVEL.get(levelname.upper(), LogLevel.INFO.value)


def format_bracketed_log_line(record: logging.LogRecord) -> str:
    """Format a `LogRecord` using the canonical Satya plain-text bracketed prefix (Section 3.4):

    `[2026-11-02T22:10:28.112Z] [INFO] [session=...] [trace=... span=...<-...] [lineage=...<-...] logger: msg`
    """
    dt = datetime.fromtimestamp(record.created, tz=timezone.utc)
    iso_ts = dt.strftime("%Y-%m-%dT%H:%M:%S.") + f"{int(record.msecs):03d}Z"
    avro_level = _map_python_level_to_avro(record.levelname)

    parts = [f"[{iso_ts}]", f"[{avro_level}]"]

    session_id = _SESSION_ID_VAR.get()
    if session_id:
        parts.append(f"[session={session_id}]")

    trace_id = _TRACE_ID_VAR.get()
    span_id = _SPAN_ID_VAR.get()
    parent_span_id = _PARENT_SPAN_ID_VAR.get()
    if trace_id and span_id:
        span_edge = f"{span_id}<-{parent_span_id}" if parent_span_id else span_id
        parts.append(f"[trace={trace_id} span={span_edge}]")
    elif trace_id:
        parts.append(f"[trace={trace_id}]")

    lineage_id = _LINEAGE_ID_VAR.get()
    parent_lineages = _PARENT_LINEAGE_IDS_VAR.get()
    if lineage_id:
        lineage_edge = (
            f"{lineage_id}<-{','.join(parent_lineages)}" if parent_lineages else lineage_id
        )
        parts.append(f"[lineage={lineage_edge}]")

    raw_msg = record.getMessage()
    clean_msg = redact_sensitive_text(raw_msg)
    prefix = " ".join(parts)
    return f"{prefix} {record.name}: {clean_msg}"


class SatyaContextLogFormatter(logging.Formatter):
    """`logging.Formatter` emitting the canonical Satya bracketed plain-text log line."""

    def format(self, record: logging.LogRecord) -> str:
        return format_bracketed_log_line(record)


class SatyaTurnLogBufferHandler(logging.Handler):
    """Bounded `contextvars`-aware handler collecting `INFO`/`WARN`/`ERROR` logs during an AI GM turn."""

    def __init__(
        self,
        max_turn_logs: int = MAX_TURN_LOGS_DEFAULT,
        max_log_line_chars: int = MAX_LOG_LINE_CHARS_DEFAULT,
    ) -> None:
        super().__init__(level=logging.INFO)
        self.max_turn_logs = max_turn_logs
        self.max_log_line_chars = max_log_line_chars

    def emit(self, record: logging.LogRecord) -> None:
        if record.levelno < logging.INFO:
            return
        buf = _TURN_LOG_BUFFER_VAR.get()
        if buf is None or len(buf) >= self.max_turn_logs:
            return
        formatted_line = format_bracketed_log_line(record)
        if len(formatted_line) > self.max_log_line_chars:
            formatted_line = formatted_line[: self.max_log_line_chars - 3] + "..."
        buf.append(
            {
                "level": _map_python_level_to_avro(record.levelname),
                "span_id": _SPAN_ID_VAR.get(),
                "parent_span_id": _PARENT_SPAN_ID_VAR.get(),
                "lineage_id": _LINEAGE_ID_VAR.get(),
                "parent_lineage_ids": list(_PARENT_LINEAGE_IDS_VAR.get()),
                "log_line": formatted_line,
            }
        )


def build_tool_invocation_record(
    inv: Mapping[str, Any],
    *,
    step_index: int,
    include_legacy_aliases: bool = False,
) -> dict[str, Any]:
    """Build a canonical `McpToolInvocationRecord` dict from a tool trace mapping."""
    tier = str(
        inv.get("mcp_tier") or inv.get("server_tier") or McpServerTier.TIER_2_RULESET_ENGINE.value
    )
    server_id = str(inv.get("server_id") or inv.get("server_name") or "satya-d20-rules")
    args_json = str(inv.get("arguments_json") or inv.get("arguments_redacted") or "{}")
    summary_json = str(inv.get("result_summary_json") or inv.get("result_summary") or "")
    is_error = (
        bool(inv.get("is_error"))
        if "is_error" in inv
        else not bool(inv.get("succeeded", True))
    )
    duration_ms = int(inv.get("duration_ms") or inv.get("latency_ms") or 1)
    record: dict[str, Any] = {
        "step_index": int(inv.get("step_index", step_index)),
        "span_id": str(inv.get("span_id") or uuid.uuid4().hex[:16]),
        "mcp_tier": tier,
        "server_id": server_id,
        "tool_name": str(inv.get("tool_name") or ""),
        "arguments_json": args_json,
        "result_summary_json": summary_json,
        "is_error": is_error,
        "duration_ms": duration_ms,
    }
    if include_legacy_aliases:
        record.update(
            {
                "server_tier": tier,
                "server_name": server_id,
                "arguments_redacted": str(inv.get("arguments_redacted") or args_json),
                "result_summary": str(inv.get("result_summary") or summary_json),
                "latency_ms": duration_ms,
                "succeeded": not is_error,
            }
        )
    return record


def build_trajectory_report_payload(
    *,
    turn_id_prefix: str = "turn",
    trace_id: str | None = None,
    operating_mode: str | AiGmOperatingMode = AiGmOperatingMode.AUTONOMOUS,
    trigger_actor: Mapping[str, Any],
    trigger_event_id: str,
    trigger_summary: str,
    llm_provider: str,
    reasoning_model_id: str,
    vision_model_id: str | None = None,
    ruleset_engine_id: str = "satya-d20",
    tool_invocations: Sequence[Mapping[str, Any]] = (),
    rag_tool_names: frozenset[str] | set[str] = frozenset(),
    emitted_event_ids: Sequence[str] | None = None,
    prompt_tokens: int = 32,
    completion_tokens: int = 16,
    cached_tokens: int = 0,
    total_turn_latency_ms: int = 1,
    turn_status: str | AiGmTurnStatus = AiGmTurnStatus.COMPLETED,
    turn_logs: Sequence[Mapping[str, Any]] | None = None,
) -> dict[str, Any]:
    """Construct a strict `GmTrajectoryReportRequest` dictionary for `/api/gm/sessions/:id/trajectory`."""
    raw_mode = str(operating_mode).lower()
    norm_mode = (
        operating_mode.value
        if isinstance(operating_mode, AiGmOperatingMode)
        else (
            AiGmOperatingMode.SOLO_CO_GM.value
            if "solo" in raw_mode
            else (
                AiGmOperatingMode.CO_PILOT.value
                if "co_pilot" in raw_mode
                else AiGmOperatingMode.AUTONOMOUS.value
            )
        )
    )
    strict_tool_records = [
        build_tool_invocation_record(inv, step_index=i)
        for i, inv in enumerate(tool_invocations)
    ]
    rag_sources_used = [
        rec["tool_name"]
        for rec in strict_tool_records
        if rec["tool_name"] in rag_tool_names
    ]
    active_trace_id = trace_id or get_current_trace_id() or uuid.uuid4().hex
    resolved_emitted = (
        get_current_emitted_event_ids()
        if emitted_event_ids is None
        else [str(eid) for eid in emitted_event_ids]
    )
    resolved_logs = (
        get_current_turn_logs()
        if turn_logs is None
        else [dict(entry) for entry in turn_logs]
    )
    resolved_status = (
        turn_status.value
        if isinstance(turn_status, AiGmTurnStatus)
        else str(turn_status)
    )
    return {
        "turn_id": f"{turn_id_prefix}-{uuid.uuid4().hex[:10]}",
        "trace_id": active_trace_id,
        "operating_mode": norm_mode,
        "trigger_actor": dict(trigger_actor),
        "trigger_event_id": trigger_event_id,
        "trigger_summary": trigger_summary,
        "llm_provider": llm_provider,
        "reasoning_model_id": reasoning_model_id,
        "vision_model_id": vision_model_id,
        "ruleset_engine_id": ruleset_engine_id,
        "tool_invocations": strict_tool_records,
        "rag_sources_used": rag_sources_used,
        "emitted_event_ids": resolved_emitted,
        "prompt_tokens": int(prompt_tokens),
        "completion_tokens": int(completion_tokens),
        "cached_tokens": int(cached_tokens),
        "total_turn_latency_ms": int(total_turn_latency_ms),
        "turn_status": resolved_status,
        "turn_logs": resolved_logs,
    }
