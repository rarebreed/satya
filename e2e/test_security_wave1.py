"""Playwright + live-server Wave 1 security E2E tests (zombie WS, assets, RBAC, leave, join races)."""

from __future__ import annotations

import base64
import secrets
from collections.abc import Callable

import httpx

from e2e.helpers import SatyaClient, SatyaServerHandle, query_sqlite


def test_zombie_websockets_close_on_logout_and_session_delete(
    satya_server: SatyaServerHandle,
    make_client: Callable[[str], SatyaClient],
) -> None:
    gm_pw = secrets.token_urlsafe(12)
    player_pw = secrets.token_urlsafe(12)

    gm = make_client("gm")
    player = make_client("player")

    gm.register("e2e_gm", gm_pw, "E2E GM")
    gm.create_campaign_and_session(
        campaign_title="Sec Audit",
        session_title="Socket Gate",
        password="sock-room-pw",
        session_id="ws-revoke-room",
        discoverable=True,
    )

    player.register("e2e_player", player_pw, "E2E Player")
    player.open_lobby_if_closed()
    player.page.locator("#btn-scope-search-campaigns").click()
    player.page.locator("#directory-gm-query").fill("@e2e_gm")
    player.page.locator("#btn-search-gm-directory").click()
    player.page.locator('#directory-user-chips button:has-text("@e2e_gm")').click()
    player.page.locator('#campaign-list-pane button:has-text("Sec Audit")').click()
    player.page.locator("#lobby-step-2").wait_for(state="visible", timeout=10_000)
    player.page.locator('#session-list-pane button:has-text("Socket Gate")').click()
    player.page.locator("#lobby-step-3").wait_for(state="visible", timeout=10_000)
    player.page.locator('button:has-text("Or join with an invite password")').click()
    player.page.locator("#dir-join-password-input").fill("sock-room-pw")
    player.page.locator('#lobby-dynamic-action-body button:has-text("Join with Password")').click()

    player.wait_snapshot(
        lambda s: s.get("session_id") == "ws-revoke-room" and s.get("ws_ready_state") == 1,
        "player connected to ws-revoke-room",
    )

    # Open probe sockets via single-use /api/ws/ticket tokens to verify server-side close
    gm.page.evaluate(
        """async () => {
            const tok = localStorage.getItem("satya.auth.token");
            const active = JSON.parse(localStorage.getItem("satya.session.active"));
            const proto = location.protocol === "https:" ? "wss:" : "ws:";
            const mkTicket = async (body) => {
                const r = await fetch("/api/ws/ticket", {
                    method: "POST",
                    headers: { "Content-Type": "application/json", "Authorization": `Bearer ${tok}` },
                    body: JSON.stringify(body),
                });
                const j = await r.json();
                return j.ticket;
            };
            const [sTick, nTick] = await Promise.all([
                mkTicket({ kind: "session", join_token: active.join_token }),
                mkTicket({ kind: "notifications" }),
            ]);
            window.__probeSessWs = new WebSocket(`${proto}//${location.host}/ws/session?ticket=${encodeURIComponent(sTick)}`);
            window.__probeNotifWs = new WebSocket(`${proto}//${location.host}/ws/notifications?ticket=${encodeURIComponent(nTick)}`);
        }"""
    )
    gm.page.wait_for_function(
        "() => window.__probeSessWs.readyState === 1 && window.__probeNotifWs.readyState === 1",
        timeout=10_000,
    )

    player.page.evaluate(
        """async () => {
            const tok = localStorage.getItem("satya.auth.token");
            const active = JSON.parse(localStorage.getItem("satya.session.active"));
            const proto = location.protocol === "https:" ? "wss:" : "ws:";
            const r = await fetch("/api/ws/ticket", {
                method: "POST",
                headers: { "Content-Type": "application/json", "Authorization": `Bearer ${tok}` },
                body: JSON.stringify({ kind: "session", join_token: active.join_token }),
            });
            const j = await r.json();
            window.__probePlayerWs = new WebSocket(`${proto}//${location.host}/ws/session?ticket=${encodeURIComponent(j.ticket)}`);
        }"""
    )
    player.page.wait_for_function("() => window.__probePlayerWs.readyState === 1", timeout=10_000)

    # 1. GM logs out -> server must close both /ws/session and /ws/notifications immediately
    gm.logout()
    gm.page.wait_for_function(
        "() => window.__probeSessWs.readyState === 3 && window.__probeNotifWs.readyState === 3",
        timeout=10_000,
    )
    gm.page.evaluate(
        """() => {
            if (window.__probeSessWs.readyState === 1) {
                window.__probeSessWs.send(JSON.stringify({
                    type: "SEND_CHAT",
                    visibility: "PUBLIC",
                    recipient_user_ids: [],
                    channel_context: "OUT_OF_CHARACTER",
                    body: "ZOMBIE_POST_LOGOUT",
                    composition_time_ms: 10
                }));
            }
        }"""
    )
    gm.page.wait_for_timeout(150)
    zombie_msgs = query_sqlite(
        satya_server.db_path,
        "SELECT payload_json FROM session_event_history WHERE session_id = ? AND payload_json LIKE '%ZOMBIE_POST_LOGOUT%'",
        ("ws-revoke-room",),
    )
    assert zombie_msgs == []

    # 2. GM logs back in and deletes the session -> player's live /ws/session must close
    gm.login("e2e_gm", gm_pw)
    gm.open_lobby_if_closed()
    gm.page.locator('#campaign-list-pane button:has-text("Sec Audit")').click()
    gm.page.locator("#lobby-step-2").wait_for(state="visible", timeout=10_000)
    gm.page.locator('#session-list-pane button:has-text("Socket Gate")').click()
    gm.page.locator("#lobby-step-3").wait_for(state="visible", timeout=10_000)
    gm.page.locator('#lobby-dynamic-action-body button:has-text("Delete Session")').click()

    player.page.wait_for_function("() => window.__probePlayerWs.readyState === 3", timeout=10_000)

    gm.assert_clean()
    player.assert_clean()


def test_wave1_api_access_controls_in_live_server(
    satya_server: SatyaServerHandle,
    make_client: Callable[[str], SatyaClient],
) -> None:
    gm_pw = secrets.token_urlsafe(12)
    player_pw = secrets.token_urlsafe(12)

    gm = make_client("gm")
    player = make_client("player")
    gm.register("e2e_gm", gm_pw, "E2E GM")
    player.register("e2e_player", player_pw, "E2E Player")

    gm.create_campaign_and_session(
        campaign_title="Vault Sec",
        session_title="Access Room",
        password="access-room-pw",
        session_id="wave1-live-room",
        discoverable=True,
    )

    gm_tok = gm.page.evaluate('() => localStorage.getItem("satya.auth.token")')
    player_tok = player.page.evaluate('() => localStorage.getItem("satya.auth.token")')
    assert isinstance(gm_tok, str) and gm_tok
    assert isinstance(player_tok, str) and player_tok

    with httpx.Client(base_url=satya_server.base_url, timeout=10.0) as http:
        # 1. Character asset MIME allowlist + auth + nosniff
        char_payload = {
            "session_id": "wave1-live-room",
            "bind_to_session": False,
            "character": {
                "character_id": "e2e-char-sec-1",
                "owner_user_id": "",
                "version": 1,
                "character_name": "Sec Tester",
                "callsign_or_role": "Field Specialist",
                "age_years": 28,
                "origin_label": "Working-Class Standard (SQM 4)",
                "campaign_sqm": 5,
                "operational_heat": 0,
                "granted_access_tags": ["UNIVERSAL"],
                "stage_history": [],
                "characteristics": {
                    "spd": 10,
                    "for_stat": 10,
                    "dex": 10,
                    "kin": 10,
                    "hlt": 10,
                    "fit": 10,
                    "hgt_m": 1.75,
                    "mas_kg": 62.0,
                    "mem": 10,
                    "ana": 10,
                    "ins": 10,
                    "cre": 10,
                    "dis": 10,
                    "foc": 10,
                    "com": 10,
                    "pre": 10,
                    "elo": 10,
                    "sav": 10,
                    "bea": 10,
                    "wrm": 10,
                    "sig": 10,
                    "hea": 10,
                    "tac": 10,
                    "olf": 10,
                    "tas": 10,
                    "itu": 10,
                    "ego": 10,
                    "slf": 10,
                    "opn": 10,
                    "blg": 10,
                    "ext": 10,
                    "cnf": 10,
                    "agg": 10,
                    "trs": 10,
                    "cps": 10,
                },
                "aptitudes": {
                    "apt_qnt": 0,
                    "apt_lng": 0,
                    "apt_bio": 0,
                    "apt_mec": 0,
                    "apt_cmp": 0,
                    "apt_kin": 0,
                    "apt_fin": 0,
                    "apt_art": 0,
                    "apt_soc": 0,
                },
                "skills": {},
                "equipped_ranged_ids": [],
                "equipped_melee_codes": [],
                "equipped_armor_codes": [],
                "field_gear_ids": [],
                "hit_zones": [],
                "fatigue_level": 0,
                "exertion_marks": 0,
                "wound_shock_index": 0,
                "blood_units_lost": 0.0,
                "active_status_conditions": [],
                "morale_state": "STEADY",
                "mental_trauma": None,
                "social_composure_lost": 0,
                "social_wound": None,
                "psyche_drift_marks": 0,
                "notes": None,
                "lifepath_config_json": None,
            },
        }
        save_res = http.post(
            "/api/characters",
            headers={"Authorization": f"Bearer {gm_tok}"},
            json=char_payload,
        )
        assert save_res.status_code == 200, save_res.text

        svg_res = http.post(
            "/api/characters/e2e-char-sec-1/assets",
            headers={"Authorization": f"Bearer {gm_tok}"},
            json={
                "asset_kind": "PORTRAIT_IMAGE",
                "filename": "xss.svg",
                "mime_type": "image/svg+xml",
                "data_base64": base64.b64encode(b"<svg onload=alert(1)/>").decode("ascii"),
            },
        )
        assert svg_res.status_code == 400

        png_res = http.post(
            "/api/characters/e2e-char-sec-1/assets",
            headers={"Authorization": f"Bearer {gm_tok}"},
            json={
                "asset_kind": "PORTRAIT_IMAGE",
                "filename": "portrait.png",
                "mime_type": "image/png",
                "data_base64": base64.b64encode(b"\x89PNG\r\n\x1a\n").decode("ascii"),
            },
        )
        assert png_res.status_code == 200, png_res.text
        asset_id = png_res.json()["character"]["assets"][0]["asset_id"]

        asset_url = f"/api/characters/e2e-char-sec-1/assets/{asset_id}"
        assert http.get(asset_url).status_code == 401
        assert (
            http.get(asset_url, headers={"Authorization": f"Bearer {player_tok}"}).status_code
            == 404
        )
        owner_get = http.get(asset_url, headers={"Authorization": f"Bearer {gm_tok}"})
        assert owner_get.status_code == 200
        assert owner_get.headers.get("x-content-type-options") == "nosniff"

        # 2. Non-owner cannot overwrite another user's existing session via import
        exp_res = http.get(
            "/api/sessions/wave1-live-room/export",
            headers={"Authorization": f"Bearer {gm_tok}"},
        )
        assert exp_res.status_code == 200
        imp_res = http.post(
            "/api/sessions/import",
            headers={"Authorization": f"Bearer {player_tok}"},
            json={
                "bundle": exp_res.json(),
                "session_id_override": "wave1-live-room",
                "session_password": "pw",
                "role": "GM",
            },
        )
        assert imp_res.status_code == 403

        # 3. Non-member leave returns empty roster
        leave_res = http.post(
            "/api/sessions/leave",
            headers={"Authorization": f"Bearer {player_tok}"},
            json={"session_id": "wave1-live-room"},
        )
        assert leave_res.status_code == 200
        assert leave_res.json()["remaining_roster"] == []

        # 4. Atomic join-request transitions + duplicate pending guard
        jr_res = http.post(
            "/api/sessions/wave1-live-room/join-requests",
            headers={"Authorization": f"Bearer {player_tok}"},
            json={"requested_role": "PLAYER"},
        )
        assert jr_res.status_code == 200
        req_id = jr_res.json()["request_id"]

        dup_jr = http.post(
            "/api/sessions/wave1-live-room/join-requests",
            headers={"Authorization": f"Bearer {player_tok}"},
            json={"requested_role": "PLAYER"},
        )
        assert dup_jr.status_code == 409

        dec1 = http.post(
            f"/api/join-requests/{req_id}/decide",
            headers={"Authorization": f"Bearer {gm_tok}"},
            json={"approve": True},
        )
        assert dec1.status_code == 200

        dec2 = http.post(
            f"/api/join-requests/{req_id}/decide",
            headers={"Authorization": f"Bearer {gm_tok}"},
            json={"approve": False},
        )
        assert dec2.status_code == 409

        cancel_after = http.delete(
            f"/api/join-requests/{req_id}",
            headers={"Authorization": f"Bearer {player_tok}"},
        )
        assert cancel_after.status_code == 409

    gm.assert_clean()
    player.assert_clean()
