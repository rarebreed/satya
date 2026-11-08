"""Automatic `ollama serve` process supervisor, stdout/stderr log collector (`data/logs/ollama.log`),
and per-turn `llama.cpp` slot telemetry extractor for `satya_ml.ai_gm`.

When `pixi run serve-ai-gm` starts with `default_provider = "ollama"`:
1. Probes `http://127.0.0.1:11434/api/tags` to detect whether `ollama serve` is already running.
2. If not running (or if `--restart-ollama` is passed), launches `ollama serve` with tuned VRAM settings
   (`OLLAMA_NUM_PARALLEL=1`, `OLLAMA_FLASH_ATTENTION=1`, `OLLAMA_KV_CACHE_TYPE=q8_0`, `OLLAMA_CONTEXT_LENGTH=16384`)
   and redirects its `stdout`/`stderr` into `data/logs/ollama.log` plus an in-memory ring buffer.
3. If already running externally, tails the OS Ollama log file (e.g. `%LOCALAPPDATA%/Ollama/server.log` on Windows)
   into `data/logs/ollama.log` and the in-memory ring buffer so slot diagnostics remain available.
"""

from __future__ import annotations

import atexit
from collections import deque
from datetime import datetime, timezone
import logging
import os
from pathlib import Path
import shutil
import subprocess
import threading
import time
from typing import Any
import urllib.request

logger = logging.getLogger("satya_ml.ai_gm.ollama")

_MAX_RING_LINES = 500
_MAX_LOG_FILE_BYTES = 10 * 1024 * 1024  # 10 MiB

_RING_LOCK = threading.Lock()
_OLLAMA_LOG_RING: deque[tuple[int, str]] = deque(maxlen=_MAX_RING_LINES)
_OLLAMA_LINE_SEQ: int = 0
_MANAGED_PROCESS: subprocess.Popen[str] | None = None
_TAIL_STOP_EVENT = threading.Event()
_SUPERVISOR_STATUS: dict[str, Any] = {
    "mode": "uninitialized",
    "base_url": "http://127.0.0.1:11434",
    "pid": None,
    "log_file": None,
    "source_log_file": None,
    "env_overrides": {},
}

# High-signal llama.cpp / Ollama server substrings worth surfacing in `satya_ml.ai_gm.ollama`
_HIGH_SIGNAL_PATTERNS: tuple[str, ...] = (
    "new prompt, n_ctx_slot",
    "prompt eval time =",
    "eval time =",
    "total time =",
    "stop processing:",
    "truncated = 1",
    "forcing full prompt re-processing",
    "[GIN]",
    "level=WARN",
    "level=ERROR",
)


def _repo_root() -> Path:
    return Path(__file__).resolve().parent.parent.parent.parent


def get_logs_dir(repo_root: Path | None = None) -> Path:
    """Return `data/logs/` under the repository root, creating it if needed."""
    root = repo_root or _repo_root()
    logs_dir = root / "data" / "logs"
    logs_dir.mkdir(parents=True, exist_ok=True)
    return logs_dir


def get_ollama_log_path(repo_root: Path | None = None) -> Path:
    """Return the path to `data/logs/ollama.log`."""
    return get_logs_dir(repo_root) / "ollama.log"


def _rotate_log_if_needed(log_path: Path) -> None:
    try:
        if log_path.exists() and log_path.stat().st_size > _MAX_LOG_FILE_BYTES:
            backup = log_path.with_suffix(".log.1")
            backup.unlink(missing_ok=True)
            log_path.replace(backup)
    except OSError:
        pass


def record_ollama_log_line(raw_line: str, *, log_path: Path | None = None, forward_to_logger: bool = True) -> int:
    """Record a single line from `ollama serve` into the ring buffer, `data/logs/ollama.log`, and logger."""
    global _OLLAMA_LINE_SEQ
    clean = raw_line.rstrip("\r\n")
    if not clean.strip():
        return _OLLAMA_LINE_SEQ

    ts = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.%f")[:-3] + "Z"
    formatted = clean if clean.startswith("[") else f"[{ts}] {clean}"

    with _RING_LOCK:
        _OLLAMA_LINE_SEQ += 1
        seq = _OLLAMA_LINE_SEQ
        _OLLAMA_LOG_RING.append((seq, formatted))

    target_path = log_path or get_ollama_log_path()
    try:
        with target_path.open("a", encoding="utf-8", newline="\n") as fp:
            fp.write(formatted + "\n")
    except OSError:
        pass

    if forward_to_logger:
        if "truncated = 1" in clean or "level=ERROR" in clean:
            logger.warning("[ollama] %s", clean)
        elif any(pat in clean for pat in _HIGH_SIGNAL_PATTERNS):
            logger.debug("[ollama] %s", clean)

    return seq


def get_ollama_log_cursor() -> int:
    """Return the current sequence number of the Ollama log ring buffer."""
    with _RING_LOCK:
        return _OLLAMA_LINE_SEQ


def drain_ollama_logs_since(cursor: int, *, high_signal_only: bool = True) -> tuple[int, list[str]]:
    """Return `(new_cursor, lines)` emitted by Ollama since `cursor`."""
    with _RING_LOCK:
        new_cursor = _OLLAMA_LINE_SEQ
        lines = [line for seq, line in _OLLAMA_LOG_RING if seq > cursor]
    if high_signal_only:
        lines = [
            ln for ln in lines if any(pat in ln for pat in _HIGH_SIGNAL_PATTERNS)
        ]
    return new_cursor, lines


def get_recent_ollama_logs(limit: int = 100, *, high_signal_only: bool = False) -> dict[str, Any]:
    """Return the most recent `limit` lines from the Ollama log buffer along with supervisor metadata."""
    capped = max(1, min(_MAX_RING_LINES, int(limit)))
    with _RING_LOCK:
        all_lines = [line for _, line in _OLLAMA_LOG_RING]
        seq = _OLLAMA_LINE_SEQ
    if high_signal_only:
        all_lines = [
            ln for ln in all_lines if any(pat in ln for pat in _HIGH_SIGNAL_PATTERNS)
        ]
    return {
        "supervisor": dict(_SUPERVISOR_STATUS),
        "cursor": seq,
        "line_count": len(all_lines[-capped:]),
        "lines": all_lines[-capped:],
    }


def is_ollama_running(base_url: str = "http://127.0.0.1:11434", timeout: float = 1.0) -> bool:
    """Return `True` if an Ollama HTTP server responds at `GET {base_url}/api/tags`."""
    url = f"{base_url.rstrip('/')}/api/tags"
    try:
        req = urllib.request.Request(url, method="GET")
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            return 200 <= resp.status < 300
    except Exception:
        return False


def _find_ollama_binary() -> str | None:
    """Locate the `ollama` CLI executable on PATH or standard Windows install paths."""
    found = shutil.which("ollama")
    if found:
        return found
    if os.name == "nt":
        local_app_data = os.environ.get("LOCALAPPDATA", "")
        if local_app_data:
            candidate = Path(local_app_data) / "Programs" / "Ollama" / "ollama.exe"
            if candidate.exists():
                return str(candidate)
    return None


def _find_system_ollama_server_log() -> Path | None:
    """Locate the default external Ollama log file (`%LOCALAPPDATA%/Ollama/server.log` on Windows)."""
    if os.name == "nt":
        local_app_data = os.environ.get("LOCALAPPDATA", "")
        if local_app_data:
            cand = Path(local_app_data) / "Ollama" / "server.log"
            if cand.exists():
                return cand
    home = Path.home()
    for rel in (".ollama/logs/server.log", "Library/Logs/Ollama/server.log"):
        cand = home / rel
        if cand.exists():
            return cand
    return None


def build_recommended_ollama_env(
    file_cfg: dict[str, Any] | None = None,
    *,
    base_env: dict[str, str] | None = None,
) -> tuple[dict[str, str], dict[str, str]]:
    """Build `(full_env, applied_defaults)` with tuned GPU/VRAM settings for `ollama serve`."""
    cfg = file_cfg or {}
    env = dict(base_env if base_env is not None else os.environ)
    num_ctx = str(int(cfg.get("num_ctx") or 16384))
    defaults = {
        "OLLAMA_NUM_PARALLEL": str(cfg.get("ollama_num_parallel") or "1"),
        "OLLAMA_MAX_LOADED_MODELS": str(cfg.get("ollama_max_loaded_models") or "1"),
        "OLLAMA_FLASH_ATTENTION": str(cfg.get("ollama_flash_attention") or "1"),
        "OLLAMA_KV_CACHE_TYPE": str(cfg.get("ollama_kv_cache_type") or "q8_0"),
        "OLLAMA_CONTEXT_LENGTH": num_ctx,
        "OLLAMA_KEEP_ALIVE": str(cfg.get("ollama_keep_alive") or "30m"),
    }
    applied: dict[str, str] = {}
    for key, val in defaults.items():
        if not env.get(key):
            env[key] = val
        applied[key] = env[key]
    return env, applied


def _stream_pipe_worker(proc: subprocess.Popen[str], log_path: Path) -> None:
    """Background daemon thread reading `proc.stdout` line-by-line into `data/logs/ollama.log`."""
    stream = proc.stdout
    if stream is None:
        return
    try:
        for raw_line in stream:
            record_ollama_log_line(raw_line, log_path=log_path, forward_to_logger=True)
    except Exception as exc:
        logger.debug("Ollama stdout reader thread exited: %s", exc)


def _tail_external_log_worker(source_log: Path, target_log: Path) -> None:
    """Background daemon thread tailing an existing external Ollama `server.log` from `SEEK_END`."""
    try:
        with source_log.open("r", encoding="utf-8", errors="replace") as fp:
            # Read the last ~4 KB on attach so recent context is immediately available, then tail new lines
            try:
                fp.seek(0, os.SEEK_END)
                end_pos = fp.tell()
                fp.seek(max(0, end_pos - 4096), os.SEEK_SET)
                if fp.tell() > 0:
                    fp.readline()  # discard partial line
                for init_line in fp.readlines()[-30:]:
                    record_ollama_log_line(init_line, log_path=target_log, forward_to_logger=False)
            except OSError:
                fp.seek(0, os.SEEK_END)

            while not _TAIL_STOP_EVENT.is_set():
                line = fp.readline()
                if line:
                    record_ollama_log_line(line, log_path=target_log, forward_to_logger=True)
                else:
                    time.sleep(0.25)
    except Exception as exc:
        logger.debug("External Ollama log tailer exited: %s", exc)


def shutdown_managed_ollama() -> None:
    """Terminate the managed `ollama serve` child process if one was spawned by this server."""
    global _MANAGED_PROCESS
    _TAIL_STOP_EVENT.set()
    proc = _MANAGED_PROCESS
    if proc is not None and proc.poll() is None:
        logger.info("Stopping managed `ollama serve` subprocess (pid=%s)...", proc.pid)
        try:
            proc.terminate()
            proc.wait(timeout=3.0)
        except Exception:
            try:
                proc.kill()
            except Exception:
                pass
    _MANAGED_PROCESS = None


atexit.register(shutdown_managed_ollama)


def ensure_ollama_running_and_logged(
    base_url: str = "http://127.0.0.1:11434",
    file_cfg: dict[str, Any] | None = None,
    *,
    restart_if_running: bool = False,
    wait_seconds: float = 8.0,
) -> dict[str, Any]:
    """Ensure `ollama serve` is running and redirect/collect its logs into `data/logs/ollama.log`.

    - If Ollama is NOT running (or `restart_if_running=True`), spawns `ollama serve` as a managed
      child process with tuned VRAM environment variables and pipes `stdout`/`stderr` directly to
      `data/logs/ollama.log` and the in-memory ring buffer.
    - If Ollama IS already running externally, attaches to it and tails `%LOCALAPPDATA%/Ollama/server.log`
      (if present) into `data/logs/ollama.log`.
    """
    global _MANAGED_PROCESS
    log_path = get_ollama_log_path()
    _rotate_log_if_needed(log_path)

    already_up = is_ollama_running(base_url=base_url, timeout=1.0)
    if already_up and restart_if_running and os.name == "nt":
        logger.info("Restarting existing `ollama` processes to capture stdout/stderr directly...")
        subprocess.run(
            ["taskkill", "/F", "/IM", "ollama.exe", "/T"],
            capture_output=True,
            text=True,
            check=False,
        )
        time.sleep(1.0)
        already_up = is_ollama_running(base_url=base_url, timeout=0.8)

    if already_up:
        sys_log = _find_system_ollama_server_log()
        _SUPERVISOR_STATUS.update(
            {
                "mode": "attached_external",
                "base_url": base_url,
                "pid": None,
                "log_file": str(log_path),
                "source_log_file": str(sys_log) if sys_log else None,
                "env_overrides": {},
            }
        )
        if sys_log is not None:
            _TAIL_STOP_EVENT.clear()
            t = threading.Thread(
                target=_tail_external_log_worker,
                args=(sys_log, log_path),
                name="satya-ollama-log-tailer",
                daemon=True,
            )
            t.start()
            logger.info(
                "Ollama is already running at %s — tailing %s -> %s",
                base_url,
                sys_log,
                log_path,
            )
        else:
            logger.info(
                "Ollama is already running at %s (pass `--restart-ollama` or stop `ollama serve` before running `pixi run serve-ai-gm` to capture raw llama.cpp slot stdout into %s)",
                base_url,
                log_path,
            )
        return dict(_SUPERVISOR_STATUS)

    ollama_bin = _find_ollama_binary()
    if not ollama_bin:
        _SUPERVISOR_STATUS.update(
            {
                "mode": "binary_not_found",
                "base_url": base_url,
                "pid": None,
                "log_file": str(log_path),
                "source_log_file": None,
                "env_overrides": {},
            }
        )
        logger.warning(
            "Ollama is not reachable at %s and `ollama` executable was not found on PATH.",
            base_url,
        )
        return dict(_SUPERVISOR_STATUS)

    child_env, applied_env = build_recommended_ollama_env(file_cfg)
    logger.info(
        "Ollama is not running at %s — starting managed `%s serve` -> %s (env=%s)",
        base_url,
        ollama_bin,
        log_path,
        applied_env,
    )
    try:
        proc = subprocess.Popen(
            [ollama_bin, "serve"],
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            encoding="utf-8",
            errors="replace",
            bufsize=1,
            env=child_env,
        )
        _MANAGED_PROCESS = proc
        reader = threading.Thread(
            target=_stream_pipe_worker,
            args=(proc, log_path),
            name="satya-ollama-stdout-collector",
            daemon=True,
        )
        reader.start()

        deadline = time.perf_counter() + max(0.5, float(wait_seconds))
        while time.perf_counter() < deadline:
            if proc.poll() is not None:
                break
            if is_ollama_running(base_url=base_url, timeout=0.6):
                break
            time.sleep(0.25)

        _SUPERVISOR_STATUS.update(
            {
                "mode": "spawned_managed",
                "base_url": base_url,
                "pid": proc.pid,
                "log_file": str(log_path),
                "source_log_file": "stdout/stderr pipe",
                "env_overrides": applied_env,
            }
        )
        logger.info(
            "Managed `ollama serve` running (pid=%d, log_file=%s)",
            proc.pid,
            log_path,
        )
    except Exception as exc:
        logger.error("Failed to launch managed `ollama serve`: %s", exc)
        _SUPERVISOR_STATUS.update(
            {
                "mode": "spawn_error",
                "base_url": base_url,
                "pid": None,
                "log_file": str(log_path),
                "error": str(exc),
            }
        )

    return dict(_SUPERVISOR_STATUS)


def format_ollama_response_telemetry(response: Any) -> str | None:
    """Extract per-call token and timing telemetry (`prompt_eval_count`, `eval_count`, `done_reason`, tok/s) from a `ChatOllama` `AIMessage`."""
    meta = getattr(response, "response_metadata", None)
    if not isinstance(meta, dict) or not meta:
        return None
    done_reason = meta.get("done_reason")
    p_tok = meta.get("prompt_eval_count")
    p_dur_ns = meta.get("prompt_eval_duration")
    e_tok = meta.get("eval_count")
    e_dur_ns = meta.get("eval_duration")
    if p_tok is None and e_tok is None and not done_reason:
        return None

    parts: list[str] = []
    if done_reason:
        parts.append(f"done_reason={done_reason}")
    if isinstance(p_tok, int):
        if isinstance(p_dur_ns, (int, float)) and p_dur_ns > 0:
            p_ms = p_dur_ns / 1_000_000.0
            p_tps = p_tok / (p_dur_ns / 1_000_000_000.0)
            parts.append(f"prompt_tokens={p_tok} ({p_ms:.0f}ms, {p_tps:.1f} tok/s)")
        else:
            parts.append(f"prompt_tokens={p_tok}")
    if isinstance(e_tok, int):
        if isinstance(e_dur_ns, (int, float)) and e_dur_ns > 0:
            e_ms = e_dur_ns / 1_000_000.0
            e_tps = e_tok / (e_dur_ns / 1_000_000_000.0)
            parts.append(f"eval_tokens={e_tok} ({e_ms:.0f}ms, {e_tps:.1f} tok/s)")
        else:
            parts.append(f"eval_tokens={e_tok}")
    return ", ".join(parts) if parts else None
