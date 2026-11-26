"""Security tests for Satya Python FastAPI bridges, Debug Dashboard, SSRF guards, and body size limits."""

from __future__ import annotations

from fastapi import HTTPException
from fastapi.testclient import TestClient
import pytest

from satya_ml.agent_server import verify_service_auth
from satya_ml.ai_gm.models import SessionLlmConfig, build_gm_chat_model
from satya_ml.ai_gm.server import app as gm_app
from satya_ml.ai_player.server import app as player_app
from satya_ml.debug_dashboard import GRAPH_HUB
from satya_ml.signaling_client import gm_service_token, is_loopback_url


def test_fail_closed_service_auth_when_token_unconfigured(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.delenv("SATYA_AI_GM_SERVICE_TOKEN", raising=False)
    monkeypatch.delenv("SATYA_GM_SERVICE_TOKEN", raising=False)
    monkeypatch.setattr("satya_ml.signaling_client.load_ai_gm_toml_config", lambda: {"service_token": ""})

    assert gm_service_token() == ""
    with pytest.raises(HTTPException) as exc_info:
        verify_service_auth("Bearer satya_gm_secret_dev_token")
    assert exc_info.value.status_code == 503

    monkeypatch.setenv("SATYA_AI_GM_SERVICE_TOKEN", "super-strong-bridge-secret")
    with pytest.raises(HTTPException) as bad_tok:
        verify_service_auth("Bearer wrong-secret")
    assert bad_tok.value.status_code == 401
    verify_service_auth("Bearer super-strong-bridge-secret")


def test_debug_dashboard_auth_csrf_headers_and_sse_cap(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("SATYA_AI_GM_SERVICE_TOKEN", "dbg-test-secret-123")
    client = TestClient(gm_app)

    # HTML & static assets carry CSP + nosniff + X-Frame-Options headers
    idx = client.get("/debug/")
    assert idx.status_code == 200
    assert idx.headers.get("x-frame-options") == "DENY"
    assert idx.headers.get("x-content-type-options") == "nosniff"
    assert "script-src 'self'" in (idx.headers.get("content-security-policy") or "")

    # /debug/api/* requires bridge/debug token
    assert client.get("/debug/api/topology").status_code == 401
    assert (
        client.get("/debug/api/topology", headers={"Authorization": "Bearer wrong"}).status_code == 401
    )
    assert (
        client.get(
            "/debug/api/topology",
            headers={"Authorization": "Bearer dbg-test-secret-123"},
        ).status_code
        == 200
    )
    assert (
        client.get(
            "/debug/api/topology",
            headers={"X-Satya-Debug-Token": "dbg-test-secret-123"},
        ).status_code
        == 200
    )

    # Mutating /debug/api/* rejects cross-origin CSRF attempts even with token
    csrf_origin = client.post(
        "/debug/api/sessions/sec-csrf/reset",
        headers={
            "X-Satya-Debug-Token": "dbg-test-secret-123",
            "Origin": "https://evil.example",
        },
    )
    assert csrf_origin.status_code == 403

    csrf_fetch = client.post(
        "/debug/api/sessions/sec-csrf/reset",
        headers={
            "X-Satya-Debug-Token": "dbg-test-secret-123",
            "Sec-Fetch-Site": "cross-site",
        },
    )
    assert csrf_fetch.status_code == 403

    # SSE subscriber cap (16 max -> 429 on 17th)
    with GRAPH_HUB._lock:
        original_subs = dict(GRAPH_HUB._subscribers)
        for idx_sub in range(16):
            GRAPH_HUB._subscribers[9000 + idx_sub] = (None, None)  # type: ignore[assignment]
    try:
        over_cap = client.get(
            "/debug/api/stream",
            headers={"X-Satya-Debug-Token": "dbg-test-secret-123"},
        )
        assert over_cap.status_code == 429
    finally:
        with GRAPH_HUB._lock:
            GRAPH_HUB._subscribers = original_subs


def test_is_loopback_url_and_ssrf_model_guards(monkeypatch: pytest.MonkeyPatch) -> None:
    assert is_loopback_url("http://127.0.0.1:3000") is True
    assert is_loopback_url("http://localhost:3000/api") is True
    assert is_loopback_url("http://[::1]:3000") is True
    assert is_loopback_url("https://evil.com/?redirect=127.0.0.1") is False
    assert is_loopback_url("http://127.0.0.1.evil.com") is False

    # SSRF: metadata IP & unspecified IPs & non-loopback Ollama rejected
    for bad_ollama in (
        "http://169.254.169.254/latest/meta-data",
        "http://0.0.0.0:11434",
        "http://10.0.0.5:11434",
    ):
        with pytest.raises(ValueError, match="Untrusted"):
            build_gm_chat_model(SessionLlmConfig(provider="ollama", model_id="m", base_url=bad_ollama))

    # Cloud provider with non-official external host rejected
    with pytest.raises(ValueError, match="Untrusted"):
        build_gm_chat_model(
            SessionLlmConfig(
                provider="openai",
                model_id="gpt-4o",
                base_url="https://evil.example/v1",
                api_key="sk-byok",
            )
        )

    # Cloud provider pointing at loopback proxy must NEVER forward server OPENAI_API_KEY env var
    monkeypatch.setenv("OPENAI_API_KEY", "sk-server-env-secret")
    with pytest.raises(ValueError, match="Explicit BYOK api_key"):
        build_gm_chat_model(
            SessionLlmConfig(
                provider="openai",
                model_id="gpt-4o",
                base_url="http://127.0.0.1:9999/v1",
                api_key=None,
            )
        )


def test_body_size_limit_middleware_rejects_oversized_requests(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("SATYA_AI_GM_SERVICE_TOKEN", "body-limit-secret")
    oversized = b"x" * (4 * 1024 * 1024 + 512)
    headers = {
        "Authorization": "Bearer body-limit-secret",
        "Content-Type": "application/json",
    }

    gm_resp = TestClient(gm_app).post("/internal/ai-gm/events", content=oversized, headers=headers)
    assert gm_resp.status_code == 413

    player_resp = TestClient(player_app).post(
        "/internal/ai-player/events", content=oversized, headers=headers
    )
    assert player_resp.status_code == 413
