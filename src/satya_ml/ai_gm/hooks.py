"""Hook Weaver & Party Motivational Manifest compiler for the Satya AI GM (`satya_ml.ai_gm.hooks`).

Wraps the deterministic Rust compiler (`satya_py.compile_party_motivational_hooks_json`)
to surface PC Psyche polarity tensions, Skill Spotlights, Operational Heat, Credential Access,
and Party Entanglement Wedges into `sync_session_context` and `ScenarioDirector`.
"""

from __future__ import annotations

import json
from typing import Any

from satya_py.campaign import (
    compile_party_motivational_hooks as _rust_compile_party_hooks,
    normalize_character_sheet_record,
)

from satya_ml.campaign_config import get_active_campaign_pack_name


def compile_player_motivational_hooks(
    characters: list[dict[str, Any]],
    pack_id: str | None = None,
    active_location: str | None = None,
) -> dict[str, Any]:
    """Compile bound PC CharacterSheetRecords into a `PartyHookManifest` dictionary via Rust."""
    resolved_pack = (pack_id or get_active_campaign_pack_name()).strip().lower() or "awakened"
    clean_chars = [
        normalize_character_sheet_record(dict(c), i)
        for i, c in enumerate(characters or [])
        if isinstance(c, dict)
    ]
    if not clean_chars:
        return {
            "pack_id": resolved_pack,
            "active_location": active_location,
            "character_profiles": {},
            "hooks": [],
            "party_entanglements": [],
        }

    scene_ctx = (
        {"location": str(active_location), "active_location": str(active_location)}
        if active_location
        else None
    )
    return _rust_compile_party_hooks(clean_chars, scene_ctx, resolved_pack)


def format_party_hooks_for_prompt(manifest: dict[str, Any], max_hooks: int = 4) -> str:
    """Format a compact (`<= 900` chars) digest of compiled PC hooks & party wedges."""
    hooks = sorted(
        list(manifest.get("hooks") or []),
        key=lambda h: float(h.get("salience", 0.0)),
        reverse=True,
    )
    entanglements = list(manifest.get("party_entanglements") or [])
    if not hooks and not entanglements:
        return ""

    parts: list[str] = ["[Party Motivational Hooks & Entanglements]"]
    for h in hooks[: max(1, int(max_hooks))]:
        cat = str(h.get("category", "HOOK"))
        title = str(h.get("title", "Hook"))
        setup = str(h.get("setup", "")).strip()
        parts.append(f"- [{cat}] {title}: {setup}")
    for ent in entanglements[:2]:
        parts.append(
            f"- [PARTY_WEDGE] {ent.get('title', 'Friction')} ({ent.get('tension_axis', '')}): "
            f"{ent.get('shared_scene_prompt', '')}"
        )
    digest = "\n".join(parts)
    if len(digest) > 900:
        return digest[:897].rstrip() + "..."
    return digest
