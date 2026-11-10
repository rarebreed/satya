"""Isolated LangGraph AI Player Agent (`satya_ml.ai_player`) for Satya TTRPG testing and multi-player simulation.

Reuses multi-provider LLM instantiation (`satya_ml.ai_gm.models`), W3C telemetry (`satya_ml.ai_gm.telemetry`),
and authoritative Satya `2d20` rules/tactical math while enforcing a strict player-only visibility boundary
(never accessing `GM_ONLY` documents, GM memory notes, or NPC secrets).
"""

from satya_ml.ai_player.graph import AiPlayerSessionState, build_ai_player_graph
from satya_ml.ai_player.tools import (
    get_builtin_player_tools,
    get_player_session_context,
    inspect_public_tactical_map,
    read_my_character_sheet,
    roll_player_d20_action,
    send_player_chat,
    set_player_session_context,
)

__all__ = [
    "AiPlayerSessionState",
    "build_ai_player_graph",
    "get_builtin_player_tools",
    "get_player_session_context",
    "inspect_public_tactical_map",
    "read_my_character_sheet",
    "roll_player_d20_action",
    "send_player_chat",
    "set_player_session_context",
]
