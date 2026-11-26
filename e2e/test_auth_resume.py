"""E2E tests for blank auth/session defaults, logout, reload, and passwordless owner resume."""

from __future__ import annotations

import secrets
from collections.abc import Callable

from e2e.helpers import SatyaClient, SatyaServerHandle, query_sqlite


def test_login_form_starts_blank_and_snapshot_omits_secrets(
    make_client: Callable[[str], SatyaClient],
) -> None:
    client = make_client("anon")

    assert client.page.locator("#auth-username").input_value() == ""
    assert client.page.locator("#auth-password").input_value() == ""
    assert client.page.locator("#auth-display-name").input_value() == ""
    assert client.page.locator("#session-password-input").input_value() == ""

    snap = client.snapshot()
    assert snap["logged_in"] is False
    assert snap["session_id"] is None
    for secret_key in ("auth_token", "join_token", "google_access_token", "password"):
        assert secret_key not in snap

    client.assert_clean()


def test_owner_create_logout_reload_resume_without_password(
    satya_server: SatyaServerHandle,
    make_client: Callable[[str], SatyaClient],
) -> None:
    gm_pw = secrets.token_urlsafe(12)
    gm = make_client("gm")
    gm.register("e2e_gm", gm_pw, "E2E GM")

    created = gm.create_campaign_and_session(
        campaign_title="DAISE",
        session_title="Awakened V1",
        password="custom-owner-secret-99",
        session_id="daise-awakened-v1",
    )
    assert created["session_id"] == "daise-awakened-v1"
    assert created["ws_ready_state"] == 1

    rows = query_sqlite(
        satya_server.db_path,
        "SELECT session_id, host_user_id FROM saved_sessions WHERE session_id = ?",
        ("daise-awakened-v1",),
    )
    assert len(rows) == 1

    gm.logout()
    gm.goto_app()
    gm.login("e2e_gm", gm_pw)

    # Select DAISE -> Awakened V1 -> Join / Resume Session (no password box on Step 3)
    gm.open_lobby_if_closed()
    gm.page.locator('#campaign-list-pane button:has-text("DAISE")').click()
    gm.page.locator("#lobby-step-2").wait_for(state="visible", timeout=10_000)
    gm.page.locator('#session-list-pane button:has-text("Awakened V1")').click()
    gm.page.locator("#lobby-step-3").wait_for(state="visible", timeout=10_000)
    gm.page.locator('#lobby-dynamic-action-body button:has-text("Join / Resume Session")').click()

    resumed = gm.wait_snapshot(
        lambda s: s.get("session_id") == "daise-awakened-v1" and s.get("ws_ready_state") == 1,
        "owner passwordless session resume",
    )
    assert resumed["self_role"] == "Gm"
    feedback_text = gm.page.locator("#session-feedback").inner_text()
    assert "Invalid session password" not in feedback_text

    gm.assert_clean()
