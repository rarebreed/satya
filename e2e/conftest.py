"""Pytest fixtures for isolated Playwright grey-box E2E tests against `satya-server`."""

from __future__ import annotations

import os
import re
import socket
import subprocess
import sys
import time
from collections.abc import Callable, Generator
from pathlib import Path

import httpx
import pytest
from playwright.sync_api import Browser, sync_playwright

REPO_ROOT = Path(__file__).resolve().parent.parent
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))

from e2e.helpers import SatyaClient, SatyaServerHandle  # noqa: E402


def _free_tcp_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return int(sock.getsockname()[1])


def _write_isolated_storage_config(tmp_path: Path) -> tuple[Path, Path]:
    """Copies `config/storage.toml` with isolated SQLite DB and topic paths inside `tmp_path`."""
    src_cfg = (REPO_ROOT / "config" / "storage.toml").read_text(encoding="utf-8")
    db_path = tmp_path / "data" / "auth" / "users.db"
    db_path.parent.mkdir(parents=True, exist_ok=True)
    db_url = f"sqlite://{db_path.as_posix()}?mode=rwc"

    rewritten = re.sub(
        r'^database_url\s*=\s*".*"$',
        f'database_url = "{db_url}"',
        src_cfg,
        flags=re.MULTILINE,
    )
    rewritten = re.sub(
        r'^bootstrap_admins\s*=\s*\[.*\]$',
        'bootstrap_admins = ["e2e_gm"]',
        rewritten,
        flags=re.MULTILINE,
    )
    rewritten = re.sub(
        r'^bootstrap_gms\s*=\s*\[.*\]$',
        'bootstrap_gms = ["e2e_gm"]',
        rewritten,
        flags=re.MULTILINE,
    )
    cfg_path = tmp_path / "storage.toml"
    cfg_path.write_text(rewritten, encoding="utf-8")
    return cfg_path, db_path


@pytest.fixture(scope="session")
def satya_server_binary() -> Path:
    """Ensures `target/e2e/satya-server` exists, building it offline if missing."""
    override = os.environ.get("SATYA_E2E_SERVER_BIN", "").strip()
    if override:
        bin_path = Path(override)
        assert bin_path.exists(), f"SATYA_E2E_SERVER_BIN not found at {bin_path}"
        return bin_path

    exe_name = "satya-server.exe" if os.name == "nt" else "satya-server"
    bin_path = REPO_ROOT / "target" / "e2e" / exe_name
    if not bin_path.exists():
        subprocess.run(
            [
                "cargo",
                "build",
                "--offline",
                "--profile",
                "e2e",
                "-p",
                "satya-signaling",
                "--bin",
                "satya-server",
            ],
            cwd=REPO_ROOT,
            check=True,
        )
    assert bin_path.exists(), f"Missing satya-server binary at {bin_path}"
    return bin_path


@pytest.fixture(scope="session")
def pkg_e2e_dir() -> Path:
    """Validates that the `e2e-hooks` Wasm bundle was compiled into `pkg-e2e/`."""
    pkg_dir = REPO_ROOT / "pkg-e2e"
    js_entry = pkg_dir / "satya_web.js"
    assert js_entry.exists(), (
        f"Missing {js_entry}. Build the e2e Wasm bundle first via: "
        "pixi run -e e2e build-wasm-e2e"
    )
    return pkg_dir


@pytest.fixture(scope="session")
def browser() -> Generator[Browser, None, None]:
    """Launches headless Chromium configured for software WebGPU and fake WebRTC media."""
    with sync_playwright() as pw:
        br = pw.chromium.launch(
            headless=True,
            args=[
                "--enable-unsafe-webgpu",
                "--use-angle=swiftshader",
                "--use-fake-device-for-media-stream",
                "--use-fake-ui-for-media-stream",
            ],
        )
        try:
            yield br
        finally:
            br.close()


@pytest.fixture
def satya_server(
    tmp_path: Path,
    satya_server_binary: Path,
    pkg_e2e_dir: Path,
) -> Generator[SatyaServerHandle, None, None]:
    """Starts an isolated `satya-server` on an ephemeral port backed by `tmp_path`."""
    port = _free_tcp_port()
    cfg_path, db_path = _write_isolated_storage_config(tmp_path)
    base_url = f"http://127.0.0.1:{port}"

    env = os.environ.copy()
    env.update(
        {
            "PORT": str(port),
            "SATYA_HOST": "127.0.0.1",
            "SATYA_DATA_DIR": str(tmp_path),
            "SATYA_STORAGE_CONFIG": str(cfg_path),
            "SATYA_PKG_DIR": str(pkg_e2e_dir),
            "RUST_LOG": "warn,satya_signaling=info",
        }
    )

    log_path = tmp_path / "satya-server.log"
    with log_path.open("w", encoding="utf-8") as log_file:
        proc = subprocess.Popen(
            [str(satya_server_binary)],
            cwd=REPO_ROOT,
            env=env,
            stdout=log_file,
            stderr=subprocess.STDOUT,
        )
        try:
            deadline = time.monotonic() + 15.0
            ready = False
            while time.monotonic() < deadline:
                if proc.poll() is not None:
                    raise RuntimeError(
                        f"satya-server exited early ({proc.returncode}):\n"
                        f"{log_path.read_text(encoding='utf-8', errors='replace')}"
                    )
                try:
                    resp = httpx.get(f"{base_url}/api/rules", timeout=1.0)
                    if resp.status_code == 200:
                        ready = True
                        break
                except httpx.HTTPError:
                    pass
                time.sleep(0.1)

            if not ready:
                raise RuntimeError(
                    f"Timed out waiting for satya-server at {base_url}:\n"
                    f"{log_path.read_text(encoding='utf-8', errors='replace')}"
                )

            yield SatyaServerHandle(
                base_url=base_url,
                db_path=db_path,
                data_dir=tmp_path,
                port=port,
            )
        finally:
            proc.terminate()
            try:
                proc.wait(timeout=5.0)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait(timeout=5.0)


@pytest.hookimpl(hookwrapper=True, tryfirst=True)
def pytest_runtest_makereport(item: pytest.Item, call: pytest.CallInfo[object]) -> Generator[None, None, None]:
    """Attaches test phase outcomes to `item` so client teardown saves failure traces."""
    outcome = yield
    rep = outcome.get_result()
    setattr(item, f"rep_{rep.when}", rep)


@pytest.fixture
def make_client(
    request: pytest.FixtureRequest,
    browser: Browser,
    satya_server: SatyaServerHandle,
) -> Generator[Callable[[str], SatyaClient], None, None]:
    """Factory creating isolated Playwright browser contexts with tracing on failure."""
    clients: list[SatyaClient] = []

    def _factory(label: str = "user") -> SatyaClient:
        ctx = browser.new_context(viewport={"width": 1440, "height": 900})
        ctx.tracing.start(screenshots=True, snapshots=True, sources=False)
        client = SatyaClient.attach(label=label, context=ctx, base_url=satya_server.base_url)
        client.goto_app()
        clients.append(client)
        return client

    try:
        yield _factory
    finally:
        rep_call = getattr(request.node, "rep_call", None)
        failed = bool(rep_call and rep_call.failed)
        results_dir = REPO_ROOT / "e2e" / "test-results"
        if failed:
            results_dir.mkdir(parents=True, exist_ok=True)
        safe_node = re.sub(r"[^A-Za-z0-9_.-]+", "_", request.node.name)
        for client in clients:
            try:
                if failed:
                    trace_path = results_dir / f"{safe_node}-{client.label}-trace.zip"
                    client.context.tracing.stop(path=str(trace_path))
                else:
                    client.context.tracing.stop()
            finally:
                client.context.close()
