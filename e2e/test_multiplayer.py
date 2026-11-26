"""Multi-user Playwright E2E tests for directory join, live chat, and GM_ONLY vault isolation."""

from __future__ import annotations

import secrets
from collections.abc import Callable

from e2e.helpers import SatyaClient, SatyaServerHandle, query_sqlite


def test_gm_and_player_join_chat_and_gm_only_doc_isolation(
    satya_server: SatyaServerHandle,
    make_client: Callable[[str], SatyaClient],
) -> None:
    gm_pw = secrets.token_urlsafe(12)
    player_pw = secrets.token_urlsafe(12)

    gm = make_client("gm")
    player = make_client("player")

    gm.register("e2e_gm", gm_pw, "E2E GM")
    gm.create_campaign_and_session(
        campaign_title="Sector 7",
        session_title="Airlock Breach",
        password="vault-pass-42",
        session_id="sector7-airlock",
        discoverable=True,
    )

    player.register("e2e_player", player_pw, "E2E Player")
    player.open_lobby_if_closed()
    player.page.locator("#btn-scope-search-campaigns").click()
    player.page.locator("#directory-gm-query").fill("@e2e_gm")
    player.page.locator("#btn-search-gm-directory").click()

    player.page.locator('#directory-user-chips button:has-text("@e2e_gm")').click()
    player.page.locator('#campaign-list-pane button:has-text("Sector 7")').click()
    player.page.locator("#lobby-step-2").wait_for(state="visible", timeout=10_000)
    player.page.locator('#session-list-pane button:has-text("Airlock Breach")').click()
    player.page.locator("#lobby-step-3").wait_for(state="visible", timeout=10_000)

    player.page.locator('button:has-text("Or join with an invite password")').click()
    player.page.locator("#dir-join-password-input").fill("vault-pass-42")
    player.page.locator('#lobby-dynamic-action-body button:has-text("Join with Password")').click()

    player.wait_snapshot(
        lambda s: s.get("session_id") == "sector7-airlock" and s.get("ws_ready_state") == 1,
        "player joined via directory password",
    )
    gm.wait_snapshot(
        lambda s: len(s.get("roster_user_ids", [])) >= 2,
        "gm roster sees joined player",
    )

    # Public chat relays over WebSocket to player
    gm.page.locator("#chat-input").fill("Hold the perimeter")
    gm.page.locator('#chat-card button:has-text("Send")').click()
    player.page.locator('#chat-log:has-text("Hold the perimeter")').wait_for(
        state="visible", timeout=10_000
    )

    # GM submits a GM_ONLY doc with a canary secret; player must never receive it in DOM, state, or WS frames
    canary = "CANARY_GM_SECRET_9917"
    gm.page.locator("#tab-btn-docs").click()
    gm.page.locator("#gm-doc-title").fill("Top Secret Plan")
    gm.page.locator("#gm-doc-content").fill(f"Classified briefing: {canary}")
    gm.page.locator("#btn-submit-gm-doc").click()

    gm.wait_snapshot(
        lambda s: len(s.get("gm_doc_ids", [])) >= 1,
        "gm_doc_ids updated after submit",
    )
    doc_rows = query_sqlite(
        satya_server.db_path,
        "SELECT doc_id, title FROM gm_documents WHERE session_id = ?",
        ("sector7-airlock",),
    )
    assert len(doc_rows) == 1
    assert doc_rows[0]["title"] == "Top Secret Plan"

    player_snap = player.snapshot()
    assert player_snap["gm_doc_ids"] == []
    assert canary not in player.page.content()
    assert all(canary not in frame for frame in player.ws_received)

    gm.assert_clean()
    player.assert_clean()
