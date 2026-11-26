"""Wave 2 Playwright & live HTTP/WebSocket security end-to-end tests."""

from __future__ import annotations

import http.client
import json
from typing import Any
from urllib.parse import urlparse
import uuid


def test_wave2_ssr_omits_demo_creds_sets_headers_and_guards_maps(
    satya_server: Any,
    make_client: Any,
) -> None:
    client = make_client("w2_headers")
    base = satya_server.base_url.rstrip("/")
    req = client.page.request

    idx = req.get(f"{base}/")
    assert idx.status == 200
    html = idx.text()
    assert "satya-truth-2026" not in html
    assert "satyaFillDemo" not in html

    hdrs = {k.lower(): v for k, v in idx.headers.items()}
    assert hdrs.get("x-frame-options") == "DENY"
    assert hdrs.get("x-content-type-options") == "nosniff"
    assert hdrs.get("referrer-policy") == "no-referrer"
    assert "permissions-policy" in hdrs
    assert "content-security-policy" in hdrs

    # `/maps/*` rejects unauthenticated reads cleanly
    map_resp = req.get(f"{base}/maps/nonexistent.png")
    assert map_resp.status == 401


def test_wave2_ws_ticket_replay_cross_origin_logout_all_and_rate_limit(
    satya_server: Any,
    make_client: Any,
) -> None:
    client = make_client("w2_tickets")
    base = satya_server.base_url.rstrip("/")
    req = client.page.request

    suffix = uuid.uuid4().hex[:8]
    uname = f"sec_w2_{suffix}"
    pw = "Strong-Pass-987!"

    reg1 = req.post(
        f"{base}/api/auth/register",
        data=json.dumps(
            {
                "username": uname,
                "password": pw,
                "display_name": "Wave2 Sec",
            }
        ),
        headers={"Content-Type": "application/json"},
    )
    assert reg1.status == 200
    tok1 = reg1.json()["auth_token"]

    login2 = req.post(
        f"{base}/api/auth/login",
        data=json.dumps({"username": uname, "password": pw}),
        headers={"Content-Type": "application/json"},
    )
    assert login2.status == 200
    tok2 = login2.json()["auth_token"]

    # Create session & issue a single-use WS ticket
    sid = f"e2e-w2-sec-{suffix}"
    create_resp = req.post(
        f"{base}/api/sessions/create",
        data=json.dumps(
            {
                "session_id": sid,
                "session_password": "room-pw-sec",
                "session_mode": "TTRPG",
                "role": "GM",
                "enable_ai_gm": False,
                "is_discoverable": False,
            }
        ),
        headers={"Authorization": f"Bearer {tok1}", "Content-Type": "application/json"},
    )
    assert create_resp.status == 200
    jtok = create_resp.json()["join_token"]

    tick_resp = req.post(
        f"{base}/api/ws/ticket",
        data=json.dumps({"kind": "session", "join_token": jtok}),
        headers={"Authorization": f"Bearer {tok1}", "Content-Type": "application/json"},
    )
    assert tick_resp.status == 200
    ticket = tick_resp.json()["ticket"]

    parsed = urlparse(satya_server.base_url)
    host = parsed.hostname or "127.0.0.1"
    port = parsed.port or 80

    # 1. Cross-origin WebSocket handshake is rejected with 403 before consuming ticket
    conn_evil = http.client.HTTPConnection(host, port, timeout=5)
    try:
        conn_evil.request(
            "GET",
            f"/ws/session?ticket={ticket}",
            headers={
                "Host": parsed.netloc,
                "Origin": "https://evil.example",
                "Connection": "Upgrade",
                "Upgrade": "websocket",
                "Sec-WebSocket-Version": "13",
                "Sec-WebSocket-Key": "dGhlIHNhbXBsZSBub25jZQ==",
            },
        )
        evil_res = conn_evil.getresponse()
        assert evil_res.status == 403
    finally:
        conn_evil.close()

    # 2. Same-origin WS upgrade succeeds once (101 Switching Protocols)
    conn_ok = http.client.HTTPConnection(host, port, timeout=5)
    try:
        conn_ok.request(
            "GET",
            f"/ws/session?ticket={ticket}",
            headers={
                "Host": parsed.netloc,
                "Origin": satya_server.base_url,
                "Connection": "Upgrade",
                "Upgrade": "websocket",
                "Sec-WebSocket-Version": "13",
                "Sec-WebSocket-Key": "dGhlIHNhbXBsZSBub25jZQ==",
            },
        )
        ok_res = conn_ok.getresponse()
        assert ok_res.status == 101
    finally:
        conn_ok.close()

    # 3. Replaying the consumed ticket fails with 401
    conn_replay = http.client.HTTPConnection(host, port, timeout=5)
    try:
        conn_replay.request(
            "GET",
            f"/ws/session?ticket={ticket}",
            headers={
                "Host": parsed.netloc,
                "Origin": satya_server.base_url,
                "Connection": "Upgrade",
                "Upgrade": "websocket",
                "Sec-WebSocket-Version": "13",
                "Sec-WebSocket-Key": "dGhlIHNhbXBsZSBub25jZQ==",
            },
        )
        replay_res = conn_replay.getresponse()
        assert replay_res.status == 401
    finally:
        conn_replay.close()

    # 4. POST /api/auth/logout-all revokes both active tokens for that user
    lo_all = req.post(
        f"{base}/api/auth/logout-all",
        headers={"Authorization": f"Bearer {tok1}"},
    )
    assert lo_all.status == 200
    assert req.get(f"{base}/api/auth/me", headers={"Authorization": f"Bearer {tok1}"}).status == 401
    assert req.get(f"{base}/api/auth/me", headers={"Authorization": f"Bearer {tok2}"}).status == 401

    # 5. Per-username brute-force limiter returns 429 + Retry-After on a dedicated disposable handle
    brute_user = f"brute_{suffix}"
    got_429 = False
    for _ in range(15):
        r = req.post(
            f"{base}/api/auth/login",
            data=json.dumps({"username": brute_user, "password": "wrong-password"}),
            headers={"Content-Type": "application/json"},
        )
        if r.status == 429:
            assert "retry-after" in {k.lower(): v for k, v in r.headers.items()}
            got_429 = True
            break
        assert r.status == 401
    assert got_429, "Expected per-username login rate limiter to trigger 429"
