"""FastAPI service (`127.0.0.1:8090`) bridging `satya-signaling` to the LangGraph AI GM."""

from __future__ import annotations

import os
from typing import Any

from fastapi import FastAPI, Header, HTTPException, Request
from langgraph.checkpoint.memory import MemorySaver
from pydantic import BaseModel, Field

from satya_ml.ai_gm.graph import build_ai_gm_graph
from satya_ml.ai_gm.models import SessionLlmConfig, redact_api_key
from satya_ml.ai_gm.tools import _gm_service_token

app = FastAPI(
    title="Satya AI GM LangGraph Service",
    version="0.1.0",
    description="Internal loopback bridge between satya-signaling (Rust) and LangGraph AI GM.",
)

_SHARED_CHECKPOINTER = MemorySaver()
_CHAT_MODEL_OVERRIDE: Any | None = None


def set_chat_model_override(model: Any | None) -> None:
    """Configure an optional chat model override for deterministic testing."""
    global _CHAT_MODEL_OVERRIDE
    _CHAT_MODEL_OVERRIDE = model


class AiGmTurnEventPayload(BaseModel):
    """Payload dispatched by `satya-signaling` (`AiGmTurnTriggerRequest`) to `POST /internal/ai-gm/events`."""

    session_id: str
    session_mode: str = "TTRPG"
    operating_mode: str = "AUTONOMOUS"
    human_gm_user_id: str | None = None
    # Support both Rust `AiGmTurnTriggerRequest` field names (`trigger_event_id`, `trigger_actor`, `context_text`)
    # and shorthand field names (`event_id`, `actor`, `content`)
    trigger_event_id: str | None = None
    event_id: str | None = None
    trigger_kind: str = "CHAT"
    trigger_actor: dict[str, Any] | None = None
    actor: dict[str, Any] = Field(default_factory=dict)
    visibility: str = "PUBLIC"
    channel: str | None = None
    context_text: str | None = None
    content: str | None = None
    roll_summary: str | None = None
    roster: list[dict[str, Any]] = Field(default_factory=list)
    provider: str | None = None
    model_id: str | None = None
    vision_model_id: str | None = None
    temperature: float | None = None
    persona_id: str | None = None
    base_url: str | None = None
    provider_config: dict[str, Any] | None = None


def _verify_service_auth(authorization: str | None) -> None:
    expected = f"Bearer {_gm_service_token()}"
    if authorization != expected:
        raise HTTPException(status_code=401, detail="Unauthorized GM bridge token")


@app.get("/health")
async def health() -> dict[str, str]:
    """Liveness check for `satya-signaling` and `pixi run serve-ai-gm`."""
    return {"status": "ok", "service": "satya-ai-gm"}


@app.post("/internal/ai-gm/events")
async def handle_ai_gm_event(
    payload: AiGmTurnEventPayload,
    request: Request,
    authorization: str | None = Header(default=None),
    x_satya_provider_key: str | None = Header(default=None),
) -> dict[str, Any]:
    """Execute one LangGraph AI GM turn for `payload.session_id`."""
    _verify_service_auth(authorization)

    p_cfg = payload.provider_config or {}
    provider = str(payload.provider or p_cfg.get("provider") or "ollama")
    model_id = str(payload.model_id or p_cfg.get("model_id") or "qwen3:8b")
    vision_model_id = payload.vision_model_id or p_cfg.get("vision_model_id") or "qwen2.5vl:7b"
    temperature = float(
        payload.temperature if payload.temperature is not None else (p_cfg.get("temperature") or 0.35)
    )
    base_url = payload.base_url or p_cfg.get("base_url") or "http://127.0.0.1:11434"

    llm_cfg = SessionLlmConfig(
        provider=provider,  # type: ignore[arg-type]
        model_id=model_id,
        vision_model_id=vision_model_id,
        temperature=temperature,
        base_url=base_url,
        api_key=x_satya_provider_key,
    )

    graph = build_ai_gm_graph(
        llm_cfg,
        chat_model=_CHAT_MODEL_OVERRIDE,
        checkpointer=_SHARED_CHECKPOINTER,
    )

    op_mode = "co_pilot" if "co_pilot" in payload.operating_mode.lower() else "autonomous"
    effective_actor = payload.trigger_actor or payload.actor
    effective_content = payload.context_text if payload.context_text is not None else payload.content
    effective_event_id = payload.trigger_event_id or payload.event_id

    initial_state = {
        "session_id": payload.session_id,
        "session_mode": payload.session_mode,
        "operating_mode": op_mode,
        "human_gm_user_id": payload.human_gm_user_id,
        "roster": payload.roster,
        "trigger_event": {
            "event_id": effective_event_id,
            "actor": effective_actor,
            "channel": payload.channel or payload.visibility,
            "content": effective_content,
            "roll_summary": payload.roll_summary,
            "operating_mode": op_mode,
            "human_gm_user_id": payload.human_gm_user_id,
        },
        "tool_invocations": [],
    }

    result = graph.invoke(
        initial_state,
        config={"configurable": {"thread_id": payload.session_id}},
    )

    return {
        "status": "ok",
        "session_id": payload.session_id,
        "operating_mode": result.get("operating_mode", op_mode),
        "provider": llm_cfg.provider,
        "model_id": llm_cfg.model_id,
        "api_key_status": redact_api_key(x_satya_provider_key),
        "final_channel": result.get("final_channel"),
        "final_reply": result.get("final_reply"),
        "tool_invocations_count": len(result.get("tool_invocations") or []),
    }


def main() -> None:
    """Run the FastAPI AI GM server via uvicorn on `127.0.0.1:8090`."""
    import uvicorn

    host = os.environ.get("SATYA_AI_GM_HOST", "127.0.0.1")
    port = int(os.environ.get("SATYA_AI_GM_PORT", "8090"))
    uvicorn.run("satya_ml.ai_gm.server:app", host=host, port=port, reload=False)


if __name__ == "__main__":
    main()
