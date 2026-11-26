"""Playwright grey-box E2E tests for TTRPG task rolls, SQLite history persistence, and reload replay."""

from __future__ import annotations

import secrets
import time
from collections.abc import Callable

from e2e.helpers import SatyaClient, SatyaServerHandle, query_sqlite


def _wait_sqlite_rows(
    server: SatyaServerHandle,
    sql: str,
    params: tuple[str, ...],
    min_count: int = 1,
    timeout_s: float = 10.0,
) -> list[dict[str, object]]:
    deadline = time.monotonic() + timeout_s
    rows: list[dict[str, object]] = []
    while time.monotonic() < deadline:
        rows = query_sqlite(server.db_path, sql, params)
        if len(rows) >= min_count:
            return rows
        time.sleep(0.05)
    raise AssertionError(f"Timed out waiting for >= {min_count} SQLite rows; got {rows}")


def test_task_roll_persists_and_replays_after_page_reload(
    satya_server: SatyaServerHandle,
    make_client: Callable[[str], SatyaClient],
) -> None:
    gm_pw = secrets.token_urlsafe(12)
    gm = make_client("gm")
    gm.register("e2e_gm", gm_pw, "E2E GM")
    gm.create_campaign_and_session(
        campaign_title="Tabletop Ops",
        session_title="Orbital Drop",
        password="tabletop-pw-77",
        session_id="tabletop-roll-room",
    )

    gm.page.locator('#ttrpg-drawer button:has-text("Roll Satya Task")').click()
    gm.page.locator("#chat-log .chat-msg").first.wait_for(state="visible", timeout=10_000)

    mech_rows = _wait_sqlite_rows(
        satya_server,
        "SELECT sequence_num, event_kind FROM session_event_history WHERE session_id = ? AND event_kind = 'MECHANICS'",
        ("tabletop-roll-room",),
        min_count=1,
    )
    assert len(mech_rows) == 1

    # Reload the browser page: localStorage auth + active session ref should auto-resume and replay the roll
    gm.goto_app()
    resumed = gm.wait_snapshot(
        lambda s: s.get("session_id") == "tabletop-roll-room" and s.get("ws_ready_state") == 1,
        "auto-resume after reload",
    )
    assert resumed["logged_in"] is True

    chat_text = gm.page.locator("#chat-log").inner_text()
    assert "Replayed saved session history" in chat_text
    assert "task checks restored" in chat_text

    gm.assert_clean()
