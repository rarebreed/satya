"""Campaign Setting Pack Loader (`awakened` default vs. `space_opera` optional).

Loads `config/campaign.toml`, `[ai_gm].campaign_pack` in `config/ai.toml`, `SATYA_CAMPAIGN_PACK`,
and starter NPC rosters from `data/npcs/*.toml` so neither `satya_ml.ai_gm` nor
`satya_ml.ai_player` hardcodes Space Opera names, factions, or presets when running the
canonical near-future Earth *Awakened* campaign.
"""

from __future__ import annotations

import copy
import os
import tomllib
from pathlib import Path
from typing import Any

_CAMPAIGN_PACK_OVERRIDE: str | None = None


def _repo_root() -> Path:
    return Path(__file__).resolve().parent.parent.parent


def _load_toml_file(path: Path) -> dict[str, Any]:
    if not path.exists():
        return {}
    try:
        data = tomllib.loads(path.read_text(encoding="utf-8"))
        return data if isinstance(data, dict) else {}
    except Exception:
        return {}


def set_campaign_pack_override(pack_name: str | None) -> None:
    """Override the active campaign pack (`'awakened'`, `'space_opera'`, or `None` to reset) for tests."""
    global _CAMPAIGN_PACK_OVERRIDE
    _CAMPAIGN_PACK_OVERRIDE = pack_name.strip().lower() if isinstance(pack_name, str) and pack_name.strip() else None


def get_active_campaign_pack_name() -> str:
    """Resolve active campaign pack name via override > env > `config/ai.toml` > `config/campaign.toml` > `'awakened'`."""
    if _CAMPAIGN_PACK_OVERRIDE:
        return _CAMPAIGN_PACK_OVERRIDE
    env_pack = os.environ.get("SATYA_CAMPAIGN_PACK", "").strip().lower()
    if env_pack:
        return env_pack
    root = _repo_root()
    ai_cfg = _load_toml_file(root / "config" / "ai.toml").get("ai_gm", {})
    if isinstance(ai_cfg, dict):
        ai_pack = str(ai_cfg.get("campaign_pack") or "").strip().lower()
        if ai_pack:
            return ai_pack
    camp_cfg = _load_toml_file(root / "config" / "campaign.toml")
    file_pack = str(camp_cfg.get("active_pack") or "").strip().lower()
    return file_pack or "awakened"


def _builtin_fallback_pack(pack_name: str) -> dict[str, Any]:
    if pack_name == "space_opera":
        return {
            "name": "space_opera",
            "title": "Aethelgard Sector 4 (Space Opera)",
            "default_sector_preset": "orbital_airlock",
            "allow_scifi_sector_presets": True,
            "lore_dirs": [],
            "npc_toml_file": "data/npcs/space_opera_npcs.toml",
            "factions": ["Aethelgard Colonial Navy", "Aethelgard Science Directorate", "Station SecOps"],
            "default_ally_token": {
                "token_id": "tok-vance",
                "entity_id": "captain_vance",
                "label": "Capt. Vance",
                "model_preset": "valkyrie_rig",
                "color_hex": "#38bdf8",
            },
            "default_hostile_token": {
                "token_id": "tok-secops-1",
                "entity_id": "npc-secops-1",
                "label": "SecOps Sentry",
                "model_preset": "secops_sentry",
                "color_hex": "#f87171",
            },
            "default_combat_spawn": {
                "entity_id": "secops_breach_lead",
                "display_name": "SecOps Breach Lead",
                "role_or_faction": "NPC — Hostile Breach Team",
                "ooda_score": 13,
                "zone_label": "Entry Breach Chokepoint",
            },
            "default_zone_labels": {"hostile": "Entry Bulkhead", "player": "Core Sector"},
        }
    return {
        "name": "awakened",
        "title": "Awakened (Near-Future Earth ~2033)",
        "default_sector_preset": "urban_crossroads",
        "allow_scifi_sector_presets": False,
        "lore_dirs": ["docs/campaign", "docs/story"],
        "npc_toml_file": "data/npcs/awakened_npcs.toml",
        "factions": ["Big Tech (Altruist / Translucent / Shen)", "Satyang", "Humanis Primaris", "Rogue AI", "DAISE"],
        "default_ally_token": {
            "token_id": "tok-ally-1",
            "entity_id": "dr_bandajara",
            "label": "Dr. Bandajara",
            "model_preset": "operative_light",
            "color_hex": "#38bdf8",
        },
        "default_hostile_token": {
            "token_id": "tok-hostile-1",
            "entity_id": "npc-secops-1",
            "label": "DAISE Tactical Agent",
            "model_preset": "secops_sentry",
            "color_hex": "#f87171",
        },
        "default_combat_spawn": {
            "entity_id": "daise_breach_lead",
            "display_name": "DAISE Containment Lead",
            "role_or_faction": "NPC — DAISE Enforcement",
            "ooda_score": 13,
            "zone_label": "Perimeter Entry",
        },
        "default_zone_labels": {"hostile": "Perimeter Entry", "player": "Interior Cover"},
    }


def get_active_campaign_pack(pack_override: str | None = None) -> dict[str, Any]:
    """Return merged metadata dictionary for the active campaign pack."""
    pack_name = (pack_override or get_active_campaign_pack_name()).strip().lower() or "awakened"
    base = _builtin_fallback_pack(pack_name)
    camp_cfg = _load_toml_file(_repo_root() / "config" / "campaign.toml")
    packs = camp_cfg.get("packs", {})
    if isinstance(packs, dict) and isinstance(packs.get(pack_name), dict):
        merged = copy.deepcopy(base)
        merged.update(copy.deepcopy(packs[pack_name]))
        merged["name"] = pack_name
        return merged
    return base


def get_pack_default_tokens(width: int, height: int, pack_override: str | None = None) -> list[dict[str, Any]]:
    """Build default starter ally + hostile map tokens calibrated to board width/height."""
    pack = get_active_campaign_pack(pack_override)
    ally = dict(pack.get("default_ally_token") or {})
    hostile = dict(pack.get("default_hostile_token") or {})
    mid_y = max(1, height // 2)
    return [
        {
            "token_id": str(ally.get("token_id") or "tok-ally-1"),
            "entity_id": str(ally.get("entity_id") or ally.get("token_id") or "tok-ally-1"),
            "label": str(ally.get("label") or "Dr. Bandajara"),
            "grid_x": min(max(1, width - 3), 2),
            "grid_y": mid_y,
            "color_hex": str(ally.get("color_hex") or "#38bdf8"),
            "model_preset": str(ally.get("model_preset") or "operative_light"),
            "facing_deg": 90,
        },
        {
            "token_id": str(hostile.get("token_id") or "tok-hostile-1"),
            "entity_id": str(hostile.get("entity_id") or "npc-secops-1"),
            "label": str(hostile.get("label") or "DAISE Tactical Agent"),
            "grid_x": max(2, width - 3),
            "grid_y": mid_y,
            "color_hex": str(hostile.get("color_hex") or "#f87171"),
            "model_preset": str(hostile.get("model_preset") or "secops_sentry"),
            "facing_deg": 270,
        },
    ]


def get_pack_combat_spawn(pack_override: str | None = None) -> dict[str, Any]:
    """Return the default hostile combat spawn descriptor for `combat_encounter` beats."""
    pack = get_active_campaign_pack(pack_override)
    return copy.deepcopy(dict(pack.get("default_combat_spawn") or _builtin_fallback_pack("awakened")["default_combat_spawn"]))


def get_pack_zone_labels(pack_override: str | None = None) -> tuple[str, str]:
    """Return `(hostile_zone_label, player_zone_label)` for the active pack."""
    pack = get_active_campaign_pack(pack_override)
    labels = pack.get("default_zone_labels") or {}
    return (
        str(labels.get("hostile") or "Perimeter Entry"),
        str(labels.get("player") or "Interior Cover"),
    )


def get_pack_lore_dirs(pack_override: str | None = None) -> list[Path]:
    """Return resolved absolute paths of lore markdown directories for the active pack."""
    pack = get_active_campaign_pack(pack_override)
    root = _repo_root()
    out: list[Path] = []
    for rel in pack.get("lore_dirs") or []:
        p = root / str(rel)
        if p.exists():
            out.append(p)
    return out


def get_pack_starter_npcs(pack_override: str | None = None) -> dict[str, dict[str, Any]]:
    """Load starter NPC profiles from the active campaign pack's `npc_toml_file`."""
    pack = get_active_campaign_pack(pack_override)
    rel = str(pack.get("npc_toml_file") or "data/npcs/awakened_npcs.toml")
    raw = _load_toml_file(_repo_root() / rel)
    out: dict[str, dict[str, Any]] = {}
    for entry in raw.get("npcs") or []:
        if not isinstance(entry, dict):
            continue
        npc_id = str(entry.get("npc_id") or "").strip().lower()
        if not npc_id:
            continue
        npc_copy = copy.deepcopy(entry)
        npc_copy["npc_id"] = npc_id
        npc_copy.setdefault("combat_stance", "NEUTRAL")
        npc_copy.setdefault("fatigue", 0)
        npc_copy.setdefault("wound_shock", 0)
        npc_copy.setdefault("disposition", "neutral")
        npc_copy.setdefault("secrets", [])
        out[npc_id] = npc_copy
    return out


def get_pack_default_ai_player(pack_override: str | None = None) -> dict[str, Any]:
    """Return canonical starter AI Player character sheet dict from the active campaign pack."""
    pack = get_active_campaign_pack(pack_override)
    sheet = copy.deepcopy(pack.get("default_ai_player") or {})
    if not sheet:
        sheet = {
            "character_id": "char-ai-mateo",
            "character_name": "Mateo Vance",
            "callsign_or_role": "Displaced Logistics Tech & Street Fixer",
            "archetype": "Displaced Logistics Tech & Street Fixer",
            "faction": "Independent / Satyang Clinic Regular",
            "characteristics": {
                "spd": 12, "for_stat": 11, "dex": 14, "kin": 12, "hlt": 12, "fit": 12,
                "mem": 12, "ana": 13, "ins": 13, "cre": 12, "dis": 13, "foc": 13,
                "com": 11, "pre": 11, "elo": 11,
                "sig": 13, "hea": 13, "tac": 12, "olf": 11, "tas": 10, "itu": 14,
                "ego": 9, "slf": 7, "opn": 14, "blg": 13, "ext": 11, "cnf": 6, "agg": 11,
            },
            "attribute_tns": {"STR": 11, "DEX": 14, "INT": 13, "WIL": 13},
            "skills": {"digital_ai_literacy": 3, "electronics_repair": 3, "persuasion_fast_talk": 2},
            "combat_stance": "NEUTRAL",
            "fatigue_level": 0,
            "wound_shock_index": 0.0,
            "loadout": ["Diagnostic Pad", "Reinforced Jacket (Guard +1)"],
            "biography": "Former automated-port tech navigating the 2033 Awakened economic crisis.",
        }
    return sheet
