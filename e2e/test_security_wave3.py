"""Wave 3 Playwright end-to-end tests: directory privacy, XSS escaping, and WebRTC/media cleanup."""

from __future__ import annotations

import json
from typing import Any
import uuid


def test_wave3_directory_privacy_and_private_join_request_blocks(
    satya_server: Any,
    make_client: Any,
) -> None:
    client = make_client("w3_privacy")
    base = satya_server.base_url.rstrip("/")
    req = client.page.request

    suffix = uuid.uuid4().hex[:8]
    hidden_user = f"w3_hidden_{suffix}"
    viewer_user = f"w3_viewer_{suffix}"
    pw = "Strong-Pass-987!"

    r_h = req.post(
        f"{base}/api/auth/register",
        data=json.dumps({"username": hidden_user, "password": pw, "display_name": "Hidden User"}),
        headers={"Content-Type": "application/json"},
    )
    assert r_h.status == 200
    tok_h = r_h.json()["auth_token"]

    r_v = req.post(
        f"{base}/api/auth/register",
        data=json.dumps({"username": viewer_user, "password": pw, "display_name": "Viewer User"}),
        headers={"Content-Type": "application/json"},
    )
    assert r_v.status == 200
    tok_v = r_v.json()["auth_token"]

    # Create non-discoverable room owned by hidden_user
    priv_sid = f"e2e-priv-{suffix}"
    cr = req.post(
        f"{base}/api/sessions/create",
        data=json.dumps(
            {
                "session_id": priv_sid,
                "session_password": "priv-room-password",
                "session_mode": "TTRPG",
                "role": "GM",
                "enable_ai_gm": False,
                "is_discoverable": False,
            }
        ),
        headers={"Authorization": f"Bearer {tok_h}", "Content-Type": "application/json"},
    )
    assert cr.status == 200

    # Non-participant viewer searching for hidden_user sees 0 results (suppressed)
    dir_other = req.get(
        f"{base}/api/directory/search?q={hidden_user}",
        headers={"Authorization": f"Bearer {tok_v}"},
    )
    assert dir_other.status == 200
    assert dir_other.json()["gms"] == []

    # Short or wildcard-only search queries return 0 results
    for q in ("ab", "%25%25%25", "___"):
        r_q = req.get(
            f"{base}/api/directory/search?q={q}",
            headers={"Authorization": f"Bearer {tok_v}"},
        )
        assert r_q.status == 200
        assert r_q.json()["gms"] == []

    # Submitting a join request to a non-discoverable room as non-participant returns 403
    jr = req.post(
        f"{base}/api/sessions/{priv_sid}/join-requests",
        data=json.dumps({"requested_role": "PLAYER"}),
        headers={"Authorization": f"Bearer {tok_v}", "Content-Type": "application/json"},
    )
    assert jr.status == 403


def test_wave3_xss_escaped_display_and_session_leave_cleanup(
    satya_server: Any,
    make_client: Any,
) -> None:
    gm = make_client("w3_xss")
    suffix = uuid.uuid4().hex[:8]
    uname = f"w3_xss_{suffix}"
    pw = "Strong-Pass-987!"

    gm.page.evaluate("() => { window.__xssTriggered = false; }")
    gm.register(uname, pw, "<img src=x onerror=\"window.__xssTriggered=true\">SafeLabel")
    gm.create_campaign_and_session(
        campaign_title="Wave 3 XSS Check",
        session_title="Safe Session",
        password="room-pw-1",
        session_id=f"e2e-xss-{suffix}",
    )

    # Confirm display name is escaped and inline onerror never ran
    assert gm.page.evaluate("() => window.__xssTriggered === false") is True

    # Leave session and verify client active state is cleaned up
    gm.page.evaluate("() => window.satyaLeaveSession()")
    gm.page.wait_for_function(
        "() => localStorage.getItem('satya.session.active') === null",
        timeout=10_000,
    )
