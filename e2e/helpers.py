"""Helpers for Playwright grey-box E2E tests against an isolated `satya-server`."""

from __future__ import annotations

import json
import sqlite3
import time
from collections.abc import Callable
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from playwright.sync_api import BrowserContext, Page


@dataclass(frozen=True)
class SatyaServerHandle:
    """Running `satya-server` instance scoped to a temporary directory."""

    base_url: str
    db_path: Path
    data_dir: Path
    port: int


def query_sqlite(
    db_path: Path,
    sql: str,
    params: tuple[Any, ...] = (),
) -> list[dict[str, Any]]:
    """Executes a read-only SQLite query against the isolated test database."""
    conn = sqlite3.connect(f"file:{db_path.as_posix()}?mode=ro", uri=True)
    conn.row_factory = sqlite3.Row
    try:
        rows = conn.execute(sql, params).fetchall()
        return [dict(r) for r in rows]
    finally:
        conn.close()


@dataclass
class SatyaClient:
    """Grey-box Playwright browser client wrapping one isolated `BrowserContext`."""

    label: str
    context: BrowserContext
    page: Page
    base_url: str
    page_errors: list[str] = field(default_factory=list)
    server_5xx: list[str] = field(default_factory=list)
    ws_received: list[str] = field(default_factory=list)

    @classmethod
    def attach(cls, label: str, context: BrowserContext, base_url: str) -> SatyaClient:
        page = context.new_page()
        client = cls(label=label, context=context, page=page, base_url=base_url)
        page.on("pageerror", lambda err: client.page_errors.append(str(err)))
        page.on(
            "response",
            lambda resp: (
                client.server_5xx.append(f"{resp.status} {resp.url}")
                if resp.status >= 500
                else None
            ),
        )

        def _on_ws(ws: Any) -> None:
            ws.on(
                "framereceived",
                lambda payload: client.ws_received.append(str(payload)),
            )

        page.on("websocket", _on_ws)
        return client

    def goto_app(self) -> None:
        """Navigates to `/` and waits until the Rust Wasm module finishes startup."""
        self.page.goto(self.base_url, wait_until="domcontentloaded")
        self.page.wait_for_function("() => window.__satyaWasmReady === true", timeout=15_000)

    def snapshot(self) -> dict[str, Any]:
        """Reads the read-only `window.__satyaE2eSnapshot()` JSON projection."""
        raw = self.page.evaluate("() => window.__satyaE2eSnapshot()")
        return json.loads(raw)

    def wait_snapshot(
        self,
        predicate: Callable[[dict[str, Any]], bool],
        desc: str,
        timeout_s: float = 12.0,
    ) -> dict[str, Any]:
        """Polls `snapshot()` until `predicate(snap)` returns True."""
        deadline = time.monotonic() + timeout_s
        last: dict[str, Any] = {}
        while time.monotonic() < deadline:
            last = self.snapshot()
            if predicate(last):
                return last
            self.page.wait_for_timeout(50)
        raise AssertionError(
            f"[{self.label}] Timed out waiting for {desc} after {timeout_s:.1f}s; "
            f"last snapshot={last}"
        )

    def register(self, username: str, password: str, display_name: str = "") -> dict[str, Any]:
        """Registers a fresh user via `#auth-card` and waits for the Wasm client to enter the lobby."""
        self.page.locator("#auth-username").fill(username)
        self.page.locator("#auth-password").fill(password)
        if display_name:
            self.page.locator("#auth-display-name").fill(display_name)
        self.page.locator('#auth-card button:has-text("Register New Account")').click()
        return self.wait_snapshot(lambda s: bool(s.get("logged_in")), f"register({username})")

    def login(self, username: str, password: str) -> dict[str, Any]:
        """Logs in via `#auth-card` and waits for the Wasm client to enter the lobby."""
        self.page.locator("#auth-username").fill(username)
        self.page.locator("#auth-password").fill(password)
        self.page.locator('#auth-card button:has-text("Login")').click()
        return self.wait_snapshot(lambda s: bool(s.get("logged_in")), f"login({username})")

    def logout(self) -> dict[str, Any]:
        """Logs out via `#btn-logout` or the hamburger menu and waits for de-auth."""
        btn_logout = self.page.locator("#btn-logout")
        if btn_logout.is_visible():
            btn_logout.click()
        else:
            self.page.locator("#btn-hamburger-menu").click()
            self.page.locator("#btn-menu-logout").click()
        return self.wait_snapshot(lambda s: not s.get("logged_in"), "logout")

    def open_lobby_if_closed(self) -> None:
        """Opens the Campaign & Session modal if it is currently hidden."""
        session_card = self.page.locator("#session-card")
        if not session_card.is_visible():
            self.page.locator("#btn-open-campaigns-modal").click()
            session_card.wait_for(state="visible", timeout=10_000)

    def create_campaign_and_session(
        self,
        campaign_title: str,
        session_title: str,
        password: str,
        *,
        mode: str = "TTRPG",
        role: str = "GM",
        session_id: str = "",
        discoverable: bool = True,
    ) -> dict[str, Any]:
        """Drives the 3-step lobby modal to create a campaign and its initial session."""
        self.open_lobby_if_closed()
        self.page.locator('#campaign-list-pane button:has-text("+ Create New Campaign")').click()
        self.page.locator("#lobby-step-2").wait_for(state="visible", timeout=10_000)

        self.page.locator("#campaign-title-input").fill(campaign_title)
        camp_cb = self.page.locator("#campaign-discoverable-checkbox")
        if camp_cb.is_checked() != discoverable:
            camp_cb.set_checked(discoverable)
        self.page.locator("#btn-create-campaign-and-session").click()
        self.page.locator("#lobby-step-3").wait_for(state="visible", timeout=10_000)

        self.page.locator("#session-title-input").fill(session_title)
        self.page.locator("#session-password-input").fill(password)
        self.page.locator("#session-mode-select").select_option(mode)
        self.page.locator("#session-role-select").select_option(role)

        if session_id or not discoverable:
            details = self.page.locator("#lobby-advanced-details")
            if details.get_attribute("open") is None:
                details.locator("summary").click()
            if session_id:
                self.page.locator("#session-id-input").fill(session_id)
            sess_cb = self.page.locator("#session-discoverable-checkbox")
            if sess_cb.is_checked() != discoverable:
                sess_cb.set_checked(discoverable)

        self.page.locator("#btn-lobby-create-submit").click()
        return self.wait_snapshot(
            lambda s: bool(s.get("session_id")) and s.get("ws_ready_state") == 1,
            f"session created & ws open ({session_title})",
        )

    def assert_clean(self) -> None:
        """Asserts that no uncaught page/Wasm errors or HTTP 5xx responses occurred."""
        assert not self.page_errors, f"[{self.label}] pageerror(s): {self.page_errors}"
        assert not self.server_5xx, f"[{self.label}] HTTP 5xx response(s): {self.server_5xx}"
