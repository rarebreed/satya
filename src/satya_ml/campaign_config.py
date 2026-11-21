"""Campaign Setting Pack Loader (`awakened` default vs. `space_opera` optional).

Loads `config/campaign.toml`, `[ai_gm].campaign_pack` in `config/ai.toml`, `SATYA_CAMPAIGN_PACK`,
starter NPC rosters from `data/npcs/*.toml`, and the compile-time embedded `config/campaign.toml`
baseline exported from Rust via `satya_py.campaign.get_embedded_campaign_config()` so Python
and Rust share one single-source truth without duplicated fallback tables.
"""

from __future__ import annotations

import copy
import os
import tomllib
from functools import lru_cache
from pathlib import Path
from typing import Any, NotRequired, TypedDict

from satya_py import campaign as _campaign

_CAMPAIGN_PACK_OVERRIDE: str | None = None


class ZoneLabelsConfig(TypedDict, total=False):
    """Hostile vs. player arrival zone labels configured on a campaign pack."""

    hostile: str
    player: str


class DefaultTokenSpec(TypedDict):
    """Starter tactical token placement emitted by `get_pack_default_tokens`."""

    token_id: str
    entity_id: str
    label: str
    grid_x: int
    grid_y: int
    color_hex: str
    model_preset: str
    facing_deg: int


class StarterScenarioConfig(TypedDict):
    """Structured Episode 1 starter premise and beat sequence returned by `get_pack_starter_scenario`."""

    episode_id: str
    title: str
    campaign_premise: str
    planned_beats: list[str]
    opening_hook: NotRequired[str]
    setting_notes: NotRequired[str]


class CampaignPackConfig(TypedDict, total=False):
    """Typed campaign pack table resolved from embedded `config/campaign.toml` + disk overrides."""

    name: str
    title: str
    default_sector_preset: str
    allow_scifi_sector_presets: bool
    lore_dirs: list[str]
    npc_toml_file: str
    default_zone_labels: ZoneLabelsConfig
    default_ally_token: dict[str, Any]
    default_hostile_token: dict[str, Any]
    default_combat_spawn: dict[str, Any]
    default_ai_player: dict[str, Any]
    starter_scenario: dict[str, Any]


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


@lru_cache(maxsize=1)
def _embedded_campaign_config() -> dict[str, Any]:
    return dict(_campaign.get_embedded_campaign_config())


def _embedded_pack(pack_name: str) -> dict[str, Any]:
    packs = _embedded_campaign_config().get("packs") or {}
    if isinstance(packs, dict):
        chosen = packs.get(pack_name) or packs.get("awakened")
        if isinstance(chosen, dict):
            result = copy.deepcopy(chosen)
            result["name"] = pack_name
            return result
    return {"name": pack_name}


def set_campaign_pack_override(pack_name: str | None) -> None:
    """Override the active campaign pack (`'awakened'`, `'space_opera'`, or `None` to reset) for tests."""
    global _CAMPAIGN_PACK_OVERRIDE
    _CAMPAIGN_PACK_OVERRIDE = pack_name.strip().lower() if isinstance(pack_name, str) and pack_name.strip() else None


def get_active_campaign_pack_name() -> str:
    """Resolve active campaign pack name via override > env > `config/ai.toml` > `config/campaign.toml` > embedded default."""
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
    if file_pack:
        return file_pack
    return str(_embedded_campaign_config().get("active_pack") or "awakened").strip().lower()


def get_active_campaign_pack(pack_override: str | None = None) -> CampaignPackConfig:
    """Return merged metadata dictionary for the active campaign pack."""
    pack_name = (pack_override or get_active_campaign_pack_name()).strip().lower() or "awakened"
    merged = _embedded_pack(pack_name)
    camp_cfg = _load_toml_file(_repo_root() / "config" / "campaign.toml")
    packs = camp_cfg.get("packs", {})
    if isinstance(packs, dict) and isinstance(packs.get(pack_name), dict):
        merged.update(copy.deepcopy(packs[pack_name]))
    merged["name"] = pack_name
    return merged  # type: ignore[return-value]


def get_pack_default_tokens(width: int, height: int, pack_override: str | None = None) -> list[DefaultTokenSpec]:
    """Build default starter ally + hostile map tokens calibrated to board width/height."""
    pack = get_active_campaign_pack(pack_override)
    ally = dict(pack.get("default_ally_token") or {})
    hostile = dict(pack.get("default_hostile_token") or {})
    mid_y = max(1, height // 2)
    return [
        DefaultTokenSpec(
            token_id=str(ally.get("token_id") or "tok-ally-1"),
            entity_id=str(ally.get("entity_id") or ally.get("token_id") or "tok-ally-1"),
            label=str(ally.get("label") or "Dr. Bandajara"),
            grid_x=min(max(1, width - 3), 2),
            grid_y=mid_y,
            color_hex=str(ally.get("color_hex") or "#38bdf8"),
            model_preset=str(ally.get("model_preset") or "operative_light"),
            facing_deg=90,
        ),
        DefaultTokenSpec(
            token_id=str(hostile.get("token_id") or "tok-hostile-1"),
            entity_id=str(hostile.get("entity_id") or "npc-secops-1"),
            label=str(hostile.get("label") or "DAISE Tactical Agent"),
            grid_x=max(2, width - 3),
            grid_y=mid_y,
            color_hex=str(hostile.get("color_hex") or "#f87171"),
            model_preset=str(hostile.get("model_preset") or "secops_sentry"),
            facing_deg=270,
        ),
    ]


def get_pack_combat_spawn(pack_override: str | None = None) -> dict[str, Any]:
    """Return the default hostile combat spawn descriptor for `combat_encounter` beats."""
    pack = get_active_campaign_pack(pack_override)
    spawn = pack.get("default_combat_spawn") or _embedded_pack("awakened").get("default_combat_spawn") or {}
    return copy.deepcopy(dict(spawn))


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
    resolved_name = str(pack.get("name") or "awakened")
    sheet = (
        pack.get("default_ai_player")
        or _embedded_pack(resolved_name).get("default_ai_player")
        or _embedded_pack("awakened").get("default_ai_player")
        or {}
    )
    return copy.deepcopy(dict(sheet))


def get_pack_starter_scenario(pack_override: str | None = None) -> StarterScenarioConfig:
    """Return the active campaign pack's non-railroaded Episode 1 premise and planned beats."""
    pack = get_active_campaign_pack(pack_override)
    resolved_name = str(pack.get("name") or "awakened")
    raw = copy.deepcopy(
        pack.get("starter_scenario")
        or _embedded_pack(resolved_name).get("starter_scenario")
        or {}
    )
    out = StarterScenarioConfig(
        episode_id=str(raw.get("episode_id") or f"{resolved_name}_ep1"),
        title=str(raw.get("title") or f"{pack.get('title', 'Awakened')} — Episode 1"),
        campaign_premise=str(raw.get("campaign_premise") or ""),
        planned_beats=[str(b) for b in (raw.get("planned_beats") or []) if str(b).strip()],
    )
    if "opening_hook" in raw and raw["opening_hook"] is not None:
        out["opening_hook"] = str(raw["opening_hook"])
    if "setting_notes" in raw and raw["setting_notes"] is not None:
        out["setting_notes"] = str(raw["setting_notes"])
    return out
