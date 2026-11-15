"""Proactive Scenario Director & Weighted Event Pacing for the Satya LangGraph AI GM.

Supports three composable flow modes (`flow_mode`):
- `"scenario_driven"`: Overarching plot milestones (`planned_beats`) adapted organically to player choices (never railroaded).
- `"free_form"`: Emergent world events sampled from player/GM-tunable probability dials (`event_weights`, `0..=100`).
- `"hybrid"` (default): Blends non-railroaded plot milestones with weighted probability sampling across 12 TTRPG event sub-routines.
"""

from __future__ import annotations

import json
import random
from typing import Any

try:
    from langchain_core.tools import tool
except ImportError:  # pragma: no cover

    def tool(fn: Any = None, **_kwargs: Any) -> Any:
        if fn is None:
            return lambda f: f
        return fn


SCENARIO_DIRECTOR_TASKS: dict[str, dict[str, Any]] = {
    "combat_encounter": {
        "slug": "combat_encounter",
        "title": "Combat & Tactical Ambush",
        "pillars": ["combat", "action", "tactical", "ambush"],
        "default_weight": 20.0,
        "summary": (
            "Outside danger erupts or breaches the scene (hostile patrol intercept, breaching charge, "
            "crossfire, or security lockdown), interrupting passive conversation and demanding immediate positioning."
        ),
        "playbook": (
            "Introduce an immediate, concrete tactical threat that reacts to where the players currently are. "
            "Call `advance_combat_tracker` or `roll_satya_d20_check` for the opening opposition check, and end with "
            "Markdown bullet choices (`- `) for tactical positioning or counter-action."
        ),
        "suggested_tools": ["advance_combat_tracker", "roll_satya_d20_check", "generate_tactical_map"],
    },
    "mysterious_stranger": {
        "slug": "mysterious_stranger",
        "title": "Mysterious Stranger Makes Their Presence Felt",
        "pillars": ["intrigue", "mystery", "espionage", "stranger"],
        "default_weight": 15.0,
        "summary": (
            "A covert informant, shadowy observer, or encrypted tight-beam transmission cuts into the scene—"
            "offering a veiled warning, blackmail leverage, or a sudden clue before vanishing or demanding an answer."
        ),
        "playbook": (
            "Interrupt the current exchange with a covert observation, encrypted comms intercept (`> **[ENCRYPTED INTERCEPT]**`), "
            "or a shadowy figure making contact. Use `lookup_npc_profile` or `whisper_to_player` if only one character notices."
        ),
        "suggested_tools": ["lookup_npc_profile", "whisper_to_player", "upsert_gm_memory"],
    },
    "new_npc_arrives": {
        "slug": "new_npc_arrives",
        "title": "New NPC Arrives & Joins the Scene",
        "pillars": ["social", "npc", "arrival", "intrigue"],
        "default_weight": 15.0,
        "summary": (
            "A new or recurring NPC physically enters the location with their own urgent agenda, interrupting "
            "a stalled conversation and shifting the social dynamic."
        ),
        "playbook": (
            "Bring an NPC into the room with an active goal or urgent news that changes the conversation's trajectory. "
            "Call `lookup_npc_profile` or `generate_npc_from_description` to ground their stats and Psyche traits."
        ),
        "suggested_tools": ["generate_npc_from_description", "lookup_npc_profile", "update_npc_state"],
    },
    "dramatic_revelation": {
        "slug": "dramatic_revelation",
        "title": "Dramatic Revelation or Plot Twist",
        "pillars": ["drama", "dramatic", "twist", "emotional"],
        "default_weight": 15.0,
        "summary": (
            "A sudden plot twist, betrayal, hidden identity reveal, or personal backstory collision raises "
            "the emotional and dramatic stakes of the current scene."
        ),
        "playbook": (
            "Reveal a high-stakes complication, double-cross, or personal connection tied to the campaign premise "
            "or a character's lifepath. Record the twist in `upsert_gm_memory`."
        ),
        "suggested_tools": ["search_gm_memory_and_docs", "lookup_npc_profile", "upsert_gm_memory"],
    },
    "thought_provoking_discussion": {
        "slug": "thought_provoking_discussion",
        "title": "Thought-Provoking Philosophical / Psyche Dilemma",
        "pillars": [
            "thought_provoking_discussion",
            "thought_provoking_discussions",
            "philosophy",
            "psyche",
            "roleplay",
            "dilemma",
        ],
        "default_weight": 10.0,
        "summary": (
            "An NPC or situation confronts the player with a hard ethical, ideological, or existential question "
            "testing their 9 bipolar Psyche traits (`EGO`, `SLF`, `OPN`, `BLG`, `EXT`, `CNF`, `AGG`, `TRS`, `CPS`)."
        ),
        "playbook": (
            "Have an NPC challenge the player's worldview or values with a concrete moral/philosophical question "
            "where there is no easy answer. Inspect `lookup_character_sheet` to tailor the question to their Psyche."
        ),
        "suggested_tools": ["lookup_character_sheet", "lookup_npc_profile", "upsert_gm_memory"],
    },
    "discovery_or_clue": {
        "slug": "discovery_or_clue",
        "title": "Discovery of a Key Clue or Anomaly",
        "pillars": ["investigation", "clue", "lore", "discovery"],
        "default_weight": 10.0,
        "summary": (
            "A physical clue, forensic trace, hidden data-slate, or overheard secret surfaces in the environment, "
            "opening a fresh investigative lead."
        ),
        "playbook": (
            "Present a tangible environmental clue or data fragment (formatted in a Markdown `> ` blockquote or table) "
            "that points toward a new location, suspect, or mystery."
        ),
        "suggested_tools": ["query_campaign_lore", "whisper_to_player", "upsert_gm_memory"],
    },
    "environmental_hazard": {
        "slug": "environmental_hazard",
        "title": "Environmental Hazard or System Failure",
        "pillars": ["survival", "exploration", "hazard", "environment"],
        "default_weight": 10.0,
        "summary": (
            "The environment turns volatile (hull depressurization, radiation spike, structural collapse, power grid "
            "failure, or bio-containment alarm), requiring immediate Physical or Mental checks."
        ),
        "playbook": (
            "Trigger an external environmental hazard (`> [!WARNING]`) that forces the crew to react immediately "
            "with a Satya `2d20` check (`roll_satya_d20_check`)."
        ),
        "suggested_tools": ["roll_satya_d20_check", "generate_tactical_map", "narrate_to_session"],
    },
    "ticking_clock_escalation": {
        "slug": "ticking_clock_escalation",
        "title": "Ticking Clock & Off-Screen Faction Escalation",
        "pillars": ["pacing", "tension", "urgency", "clock"],
        "default_weight": 10.0,
        "summary": (
            "An off-screen threat or faction countdown visibly advances—reinforcements close in, a transport prepares "
            "to undock, or an antagonist completes a phase of their plan."
        ),
        "playbook": (
            "Show visible signs that the world is moving forward without waiting on the players: broadcast a station "
            "alert, faction movement, or countdown update and log it via `upsert_gm_memory`."
        ),
        "suggested_tools": ["upsert_gm_memory", "advance_combat_tracker", "narrate_to_session"],
    },
    "heist_or_infiltration_complication": {
        "slug": "heist_or_infiltration_complication",
        "title": "Heist, Stealth, or Security Complication",
        "pillars": ["stealth", "heist", "infiltration", "complication"],
        "default_weight": 10.0,
        "summary": (
            "A guard patrol shifts early, a biometric lock cycles its cipher, a drone sweeps the corridor, or a "
            "bystander nearly spots the crew, demanding clever improvisation."
        ),
        "playbook": (
            "Introduce a tense stealth/infiltration wrinkle that rewards quick thinking, gadget use, or social bluffing "
            "before alarms sound."
        ),
        "suggested_tools": ["roll_satya_d20_check", "inspect_tactical_map", "whisper_to_player"],
    },
    "moral_or_faction_dilemma": {
        "slug": "moral_or_faction_dilemma",
        "title": "Competing Faction or Moral Dilemma",
        "pillars": ["politics", "factions", "dilemma", "social"],
        "default_weight": 10.0,
        "summary": (
            "Two rival factions or allies make competing, mutually exclusive demands of the players—helping one "
            "visibly costs standing or safety with the other."
        ),
        "playbook": (
            "Put two NPCs or faction interests in direct tension right now, forcing the players to choose who to back "
            "or attempt a risky compromise."
        ),
        "suggested_tools": ["lookup_npc_profile", "update_npc_state", "upsert_gm_memory"],
    },
    "resource_or_gear_windfall": {
        "slug": "resource_or_gear_windfall",
        "title": "Time-Sensitive Resource, Salvage, or Tactical Windfall",
        "pillars": ["loot", "salvage", "opportunity", "reward"],
        "default_weight": 5.0,
        "summary": (
            "A high-value opportunity appears—an unguarded supply cache, prototype equipment, black-market contact, "
            "or tactical vantage point—if the crew acts before the window closes."
        ),
        "playbook": (
            "Dangle a tempting, time-sensitive reward or tactical advantage that requires a bold detour or skill check to secure."
        ),
        "suggested_tools": ["roll_satya_d20_check", "upsert_gm_memory", "narrate_to_session"],
    },
    "downtime_campfire_moment": {
        "slug": "downtime_campfire_moment",
        "title": "Downtime / Campfire Character Spotlight",
        "pillars": ["bonding", "spotlight", "downtime", "roleplay"],
        "default_weight": 5.0,
        "summary": (
            "A brief quiet beat amidst tension where an ally tends to wounds/fatigue, shares a personal memory, "
            "or asks a character what they are fighting for."
        ),
        "playbook": (
            "Create a grounded, character-driven breathing space where an NPC invites personal reflection, camaraderie, "
            "or recovery before the next storm hits."
        ),
        "suggested_tools": ["lookup_character_sheet", "update_npc_state", "narrate_to_session"],
    },
}

_SESSION_SCENARIO_DIRECTORS: dict[str, dict[str, Any]] = {}


def default_event_weights() -> dict[str, float]:
    """Return a fresh copy of the default probability weights for all 12 TTRPG event sub-routines."""
    return {slug: float(spec["default_weight"]) for slug, spec in SCENARIO_DIRECTOR_TASKS.items()}


def resolve_task_slug(raw_key: str) -> str | None:
    """Map either an exact task slug or a shorthand pillar keyword (e.g. `'combat'`, `'drama'`) to a canonical slug."""
    norm = raw_key.strip().lower().replace("-", "_").replace(" ", "_")
    if not norm:
        return None
    if norm in SCENARIO_DIRECTOR_TASKS:
        return norm
    # Common alias mappings
    alias_map = {
        "combat": "combat_encounter",
        "action": "combat_encounter",
        "intrigue": "mysterious_stranger",
        "stranger": "mysterious_stranger",
        "mystery": "mysterious_stranger",
        "npc": "new_npc_arrives",
        "new_npc": "new_npc_arrives",
        "drama": "dramatic_revelation",
        "dramatic": "dramatic_revelation",
        "twist": "dramatic_revelation",
        "thought_provoking": "thought_provoking_discussion",
        "thought_provoking_discussions": "thought_provoking_discussion",
        "philosophy": "thought_provoking_discussion",
        "clue": "discovery_or_clue",
        "investigation": "discovery_or_clue",
        "hazard": "environmental_hazard",
        "world_event_interruption": "environmental_hazard",
        "clock": "ticking_clock_escalation",
        "pacing": "ticking_clock_escalation",
        "heist": "heist_or_infiltration_complication",
        "stealth": "heist_or_infiltration_complication",
        "faction": "moral_or_faction_dilemma",
        "politics": "moral_or_faction_dilemma",
        "loot": "resource_or_gear_windfall",
        "windfall": "resource_or_gear_windfall",
        "downtime": "downtime_campfire_moment",
        "campfire": "downtime_campfire_moment",
    }
    if norm in alias_map:
        return alias_map[norm]
    for slug, spec in SCENARIO_DIRECTOR_TASKS.items():
        if norm in spec.get("pillars", []):
            return slug
    return None


def normalize_event_weights(
    raw_weights: dict[str, Any] | None = None,
    player_desires: list[str] | None = None,
) -> dict[str, float]:
    """Merge user-supplied `event_weights` and `player_desires` into a complete 12-task weight table (`>= 0.0`)."""
    weights = default_event_weights()
    if player_desires:
        # Boost tasks matching explicit player desires
        for desire in player_desires:
            matched = resolve_task_slug(str(desire))
            if matched:
                weights[matched] = max(weights.get(matched, 10.0) * 1.75, 25.0)
    if isinstance(raw_weights, dict):
        for k, v in raw_weights.items():
            matched = resolve_task_slug(str(k))
            if matched is not None:
                try:
                    weights[matched] = max(0.0, float(v))
                except (TypeError, ValueError):
                    pass
    return weights


def compute_normalized_probabilities(weights: dict[str, float]) -> dict[str, float]:
    """Convert non-negative task weights into a normalized probability distribution summing to `1.0`."""
    positive_total = sum(max(0.0, float(v)) for v in weights.values())
    if positive_total <= 0.0:
        return {k: 0.0 for k in SCENARIO_DIRECTOR_TASKS}
    return {
        k: round(max(0.0, float(weights.get(k, 0.0))) / positive_total, 4)
        for k in SCENARIO_DIRECTOR_TASKS
    }


def _load_starter_scenario_defaults() -> tuple[str, list[str], str]:
    from satya_ml.campaign_config import get_active_campaign_pack_name, get_pack_starter_scenario

    active_pack = get_active_campaign_pack_name()
    starter = get_pack_starter_scenario(active_pack)
    premise = str(starter.get("campaign_premise") or "")
    beats = [str(b) for b in (starter.get("planned_beats") or []) if str(b).strip()]
    return premise, beats, active_pack


def _default_director_state() -> dict[str, Any]:
    weights = default_event_weights()
    starter_premise, starter_beats, active_pack = _load_starter_scenario_defaults()
    return {
        "flow_mode": "hybrid",
        "campaign_premise": starter_premise,
        "player_desires": ["intrigue", "combat", "thought_provoking_discussions"],
        "planned_beats": starter_beats,
        "auto_seeded_starter": bool(starter_premise or starter_beats),
        "seeded_pack": active_pack,
        "event_weights": weights,
        "normalized_probabilities": compute_normalized_probabilities(weights),
        "beat_probability_per_turn": 0.35,
        "probability_gate_active": False,
        "beat_interval_turns": 3,
        "turns_since_last_beat": 0,
        "completed_beats": [],
        "active_mandate": None,
    }


def get_session_scenario_director(session_id: str) -> dict[str, Any]:
    """Return the Scenario Director state dictionary for `session_id`, initializing defaults if absent."""
    from satya_ml.campaign_config import get_active_campaign_pack_name

    sid = session_id or "default"
    cur = _SESSION_SCENARIO_DIRECTORS.get(sid)
    if cur is None or (
        cur.get("auto_seeded_starter")
        and not cur.get("completed_beats")
        and cur.get("seeded_pack") != get_active_campaign_pack_name()
    ):
        _SESSION_SCENARIO_DIRECTORS[sid] = _default_director_state()
    return dict(_SESSION_SCENARIO_DIRECTORS[sid])


def set_session_scenario_director(session_id: str, state: dict[str, Any]) -> dict[str, Any]:
    """Update and return the Scenario Director state for `session_id`."""
    sid = session_id or "default"
    current = _SESSION_SCENARIO_DIRECTORS.get(sid) or _default_director_state()
    merged = {**current, **state}
    fm = str(merged.get("flow_mode", "hybrid")).strip().lower()
    if fm not in {"hybrid", "scenario_driven", "free_form"}:
        fm = "hybrid"
    merged["flow_mode"] = fm
    merged["beat_interval_turns"] = max(1, int(merged.get("beat_interval_turns", 3)))
    merged["beat_probability_per_turn"] = max(
        0.0, min(1.0, float(merged.get("beat_probability_per_turn", 0.35)))
    )
    merged["probability_gate_active"] = bool(merged.get("probability_gate_active", False))
    weights = normalize_event_weights(
        merged.get("event_weights"),
        merged.get("player_desires"),
    )
    merged["event_weights"] = weights
    merged["normalized_probabilities"] = compute_normalized_probabilities(weights)
    _SESSION_SCENARIO_DIRECTORS[sid] = merged
    return dict(merged)


def sync_scenario_director_from_memories(
    session_id: str,
    memories: list[dict[str, Any]] | None,
) -> dict[str, Any]:
    """Hydrate the Scenario Director state from a pinned `mem-scenario-director` note in the GM Memory Vault."""
    if not memories:
        return get_session_scenario_director(session_id)
    for m in memories:
        if not isinstance(m, dict):
            continue
        mid = str(m.get("memory_id", ""))
        title = str(m.get("title", ""))
        if mid == "mem-scenario-director" or title == "Scenario Director & Campaign Pacing Guide":
            content = str(m.get("content", "")).strip()
            if content.startswith("{"):
                try:
                    parsed = json.loads(content)
                    if isinstance(parsed, dict):
                        return set_session_scenario_director(session_id, parsed)
                except Exception:
                    pass
    return get_session_scenario_director(session_id)


def _match_task_for_planned_beat(beat_text: str, weights: dict[str, float]) -> str:
    """Infer the best matching task sub-routine for an authored `planned_beat` string."""
    lower = beat_text.lower()
    keyword_map = [
        (["ambush", "attack", "combat", "enforcer", "firefight", "breach", "sentry"], "combat_encounter"),
        (["stranger", "informant", "encrypted", "intercept", "whisper", "shadow"], "mysterious_stranger"),
        (["arrive", "enters", "joins", "envoy", "doctor", "captain"], "new_npc_arrives"),
        (["betray", "reveal", "twist", "secret", "identity"], "dramatic_revelation"),
        (["ethic", "moral", "philosophy", "question", "conscience", "psyche"], "thought_provoking_discussion"),
        (["clue", "evidence", "dossier", "trace", "beacon", "signal", "lab"], "discovery_or_clue"),
        (["hazard", "radiation", "depressur", "airlock", "vent", "collapse", "outage"], "environmental_hazard"),
        (["clock", "countdown", "lockdown", "reinforce", "undock"], "ticking_clock_escalation"),
        (["patrol", "camera", "biometric", "stealth", "heist", "alarm"], "heist_or_infiltration_complication"),
        (["faction", "syndicate", "ultimatum", "rival"], "moral_or_faction_dilemma"),
        (["cache", "salvage", "prototype", "supply", "loot"], "resource_or_gear_windfall"),
        (["rest", "campfire", "downtime", "medical", "bond"], "downtime_campfire_moment"),
    ]
    for keywords, slug in keyword_map:
        if any(kw in lower for kw in keywords) and weights.get(slug, 1.0) > 0.0:
            return slug
    # Fallback to highest-weighted enabled task
    best_slug = max(
        SCENARIO_DIRECTOR_TASKS.keys(),
        key=lambda s: float(weights.get(s, 0.0)),
    )
    return best_slug


_HOOK_CATEGORY_TO_DIRECTOR_SLUGS: dict[str, tuple[str, ...]] = {
    "IDENTITY_PSYCHE": ("thought_provoking_discussion", "moral_or_faction_dilemma", "dramatic_revelation"),
    "SKILL_SPOTLIGHT": ("discovery_or_clue", "heist_or_infiltration_complication"),
    "CREDENTIAL_ACCESS": ("heist_or_infiltration_complication", "mysterious_stranger"),
    "OPERATIONAL_HEAT": ("ticking_clock_escalation", "combat_encounter", "heist_or_infiltration_complication"),
    "LIFEPATH_DEBT": ("mysterious_stranger", "dramatic_revelation", "moral_or_faction_dilemma"),
    "CONDITION_STRAIN": ("downtime_campfire_moment", "environmental_hazard"),
    "PARTY_FRICTION": ("moral_or_faction_dilemma", "thought_provoking_discussion"),
    "FACTION_WEDGE": ("moral_or_faction_dilemma", "ticking_clock_escalation", "dramatic_revelation"),
    "HOBBY_CONTACT": ("mysterious_stranger", "thought_provoking_discussion", "downtime_campfire_moment"),
    "FORMATIVE_ANCHOR": ("dramatic_revelation", "moral_or_faction_dilemma", "downtime_campfire_moment"),
}


def _collect_hook_and_manifest_context(
    party_hooks: dict[str, Any] | None,
    narrative_manifest: dict[str, Any] | None,
    world_ledger: dict[str, Any] | None = None,
) -> tuple[set[str], dict[str, dict[str, Any]], dict[str, Any] | None, str, str]:
    """Derive boosted director slugs, per-slug top hooks, tactical zone/exit context, and world-heat summary."""
    boosted_slugs: set[str] = set()
    best_hook_for_slug: dict[str, dict[str, Any]] = {}
    top_hook: dict[str, Any] | None = None

    if isinstance(party_hooks, dict):
        hooks = sorted(
            [dict(h) for h in (party_hooks.get("hooks") or []) if isinstance(h, dict)],
            key=lambda h: float(h.get("salience", 0.0)),
            reverse=True,
        )
        if hooks:
            top_hook = hooks[0]
        for h in hooks:
            cat = str(h.get("category") or "").strip().upper()
            mapped_slugs = list(_HOOK_CATEGORY_TO_DIRECTOR_SLUGS.get(cat, ()))
            for extra in h.get("recommended_director_slugs") or []:
                slug = resolve_task_slug(str(extra))
                if slug:
                    mapped_slugs.append(slug)
            for slug in mapped_slugs:
                boosted_slugs.add(slug)
                best_hook_for_slug.setdefault(slug, h)

        entanglements = sorted(
            [dict(e) for e in (party_hooks.get("party_entanglements") or []) if isinstance(e, dict)],
            key=lambda e: float(e.get("salience", 0.0)),
            reverse=True,
        )
        for ent in entanglements:
            boosted_slugs.add("moral_or_faction_dilemma")
            boosted_slugs.add("thought_provoking_discussion")
            pseudo_card = {
                "category": "PARTY_FRICTION",
                "title": str(ent.get("title") or "Party Friction"),
                "setup": str(ent.get("shared_scene_prompt") or ""),
                "emotional_stake": str(ent.get("tension_axis") or "Competing party values"),
                "salience": float(ent.get("salience", 0.75)),
            }
            best_hook_for_slug.setdefault("moral_or_faction_dilemma", pseudo_card)
            if top_hook is None or float(pseudo_card["salience"]) > float(top_hook.get("salience", 0.0)):
                top_hook = pseudo_card

    manifest_summary = ""
    if isinstance(narrative_manifest, dict) and narrative_manifest:
        zones = [z for z in (narrative_manifest.get("zones") or []) if isinstance(z, dict)]
        exits = [e for e in (narrative_manifest.get("cardinal_exits") or []) if isinstance(e, dict)]
        if len(zones) >= 2:
            boosted_slugs.add("discovery_or_clue")
            boosted_slugs.add("heist_or_infiltration_complication")
        zone_labels = [str(z.get("label") or z.get("category") or "").strip() for z in zones[:4]]
        zone_labels = [z for z in zone_labels if z]
        exit_labels = [
            f"{e.get('direction')}: {e.get('preview_label')}"
            for e in exits[:4]
            if e.get("direction") and e.get("preview_label")
        ]
        parts: list[str] = []
        if zone_labels:
            parts.append(f"Zones: {', '.join(zone_labels)}")
        if exit_labels:
            parts.append(f"Exits: {', '.join(exit_labels)}")
        if not parts and narrative_manifest.get("concise_brief"):
            parts.append(str(narrative_manifest["concise_brief"])[:280])
        manifest_summary = " | ".join(parts)

    world_summary = ""
    if isinstance(world_ledger, dict) and world_ledger:
        heat = int(world_ledger.get("operational_heat", 0) or 0)
        tier_code = str(world_ledger.get("heat_tier_code") or "UNNOTICED")
        econ = str(world_ledger.get("active_economic_tier") or "MiddleClass")
        if heat >= 3:
            boosted_slugs.add("ticking_clock_escalation")
            boosted_slugs.add("heist_or_infiltration_complication")
            boosted_slugs.add("combat_encounter")
        urgent_clocks: list[str] = []
        for clk in world_ledger.get("faction_clocks") or []:
            if not isinstance(clk, dict):
                continue
            prog = int(clk.get("progress", 0) or 0)
            segs = max(1, int(clk.get("segments", 6) or 6))
            if prog >= max(1, segs - 1):
                boosted_slugs.add("ticking_clock_escalation")
                boosted_slugs.add("moral_or_faction_dilemma")
            urgent_clocks.append(f"{clk.get('faction_id')} {prog}/{segs}")
        clocks_str = f" | Clocks: {', '.join(urgent_clocks[:4])}" if urgent_clocks else ""
        world_summary = f"Heat {heat}/10 ({tier_code}, {econ}){clocks_str}"

    return boosted_slugs, best_hook_for_slug, top_hook, manifest_summary, world_summary


def select_next_scenario_beat(
    session_id: str,
    task_slug: str = "auto",
    custom_context: str = "",
    rng_seed: int | None = None,
    party_hooks: dict[str, Any] | None = None,
    narrative_manifest: dict[str, Any] | None = None,
    world_ledger: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Select the next Scenario Director beat according to `flow_mode`, `planned_beats`, `event_weights`, PC hooks, sector layout, and world heat/clocks.

    - In `"scenario_driven"` mode, prioritizes unconsumed `planned_beats` adapted to player actions without railroading.
    - In `"free_form"` mode, samples purely from the player/GM's `event_weights` probability dials (`w_i > 0`).
    - In `"hybrid"` mode, alternates between unconsumed `planned_beats` and weighted probability sampling.
    """
    state = get_session_scenario_director(session_id)
    flow_mode = str(state.get("flow_mode", "hybrid"))
    weights: dict[str, float] = dict(state.get("event_weights") or default_event_weights())
    planned_beats: list[str] = list(state.get("planned_beats") or [])
    completed: list[dict[str, Any]] = list(state.get("completed_beats") or [])
    completed_plot_texts = {
        str(b.get("planned_beat_text")) for b in completed if b.get("planned_beat_text")
    }
    remaining_planned = [b for b in planned_beats if b not in completed_plot_texts]
    last_task = str(completed[-1].get("task_slug", "")) if completed else ""

    boosted_slugs, best_hook_for_slug, top_hook, manifest_summary, world_summary = _collect_hook_and_manifest_context(
        party_hooks,
        narrative_manifest,
        world_ledger,
    )

    explicit = resolve_task_slug(task_slug) if task_slug and task_slug.lower() != "auto" else None
    planned_beat_used: str | None = None
    beat_source = "weighted_probability"

    if explicit is not None:
        chosen_slug = explicit
        beat_source = "explicit_override"
    elif remaining_planned and (
        flow_mode == "scenario_driven"
        or (flow_mode == "hybrid" and len(completed) % 2 == 0)
    ):
        planned_beat_used = remaining_planned[0]
        chosen_slug = _match_task_for_planned_beat(planned_beat_used, weights)
        beat_source = "planned_plot_milestone"
    else:
        # Weighted probability sample across all tasks with weight > 0 (0.0 stays strictly excluded)
        positive_items = [
            (slug, float(w))
            for slug, w in weights.items()
            if slug in SCENARIO_DIRECTOR_TASKS and float(w) > 0.0
        ]
        if not positive_items:
            # If user set every single dial to 0, fall back to a gentle world/pacing beat
            chosen_slug = "ticking_clock_escalation"
        elif len(positive_items) == 1:
            chosen_slug = positive_items[0][0]
        else:
            # Apply 0.25x cooldown on preceding task and +25% salience boost for active PC hooks / sector affordances
            adjusted: list[tuple[str, float]] = []
            for slug, w in positive_items:
                adj_w = w * 0.25 if slug == last_task else w
                if slug in boosted_slugs:
                    adj_w *= 1.25
                adjusted.append((slug, adj_w))
            total_w = sum(w for _, w in adjusted)
            rng = random.Random(rng_seed) if rng_seed is not None else random
            pick = rng.random() * total_w
            acc = 0.0
            chosen_slug = adjusted[-1][0]
            for slug, w in adjusted:
                acc += w
                if pick <= acc:
                    chosen_slug = slug
                    break

    spec = SCENARIO_DIRECTOR_TASKS[chosen_slug]
    matched_hook = best_hook_for_slug.get(chosen_slug) or top_hook
    matched_hook_title = str(matched_hook.get("title") or "").strip() if isinstance(matched_hook, dict) else ""
    premise = str(state.get("campaign_premise") or "").strip()
    anti_railroad_note = (
        "ANTI-RAILROADING RULE: Do NOT invalidate player choices or force them down a predetermined track. "
        "React to what the player just did or said, and adapt HOW and WHERE this event manifests so it collides "
        "organically with their current location and trajectory."
    )
    plot_line = (
        f"\n- Overarching Plot Milestone to Adapt: \"{planned_beat_used}\""
        if planned_beat_used
        else ""
    )
    premise_line = f"\n- Campaign Premise: {premise}" if premise else ""
    custom_line = f"\n- Additional Director Context: {custom_context.strip()}" if custom_context.strip() else ""
    hook_line = ""
    if isinstance(matched_hook, dict) and matched_hook_title:
        hook_line = (
            f"\n- Active PC Motivational Hook to Pull: [{matched_hook.get('category', 'HOOK')}] "
            f"{matched_hook_title} — {str(matched_hook.get('setup') or '').strip()} "
            f"(Stake: {str(matched_hook.get('emotional_stake') or '').strip()})"
        )
    sector_line = (
        f"\n- Tactical Sector Affordances: {manifest_summary}"
        if manifest_summary
        else ""
    )
    world_line = (
        f"\n- World Clocks & Heat: {world_summary}"
        if world_summary
        else ""
    )

    mandate_text = (
        f"[SCENARIO DIRECTOR MANDATE — {spec['title'].upper()} (`{chosen_slug}` | mode=`{flow_mode}` | source=`{beat_source}`)]"
        f"{premise_line}{plot_line}{custom_line}{hook_line}{sector_line}{world_line}\n"
        f"- World Beat Objective: {spec['summary']}\n"
        f"- Director Playbook: {spec['playbook']}\n"
        f"- Suggested Tools: {', '.join(spec['suggested_tools'])}\n"
        f"- {anti_railroad_note}\n"
        f"- Format your narration in rich Markdown (`**bold**`, `*italics*`, `> ` blockquotes for intercepts/alerts, and `- ` bulleted action hooks)."
    )

    beat_record = {
        "task_slug": chosen_slug,
        "title": spec["title"],
        "flow_mode": flow_mode,
        "beat_source": beat_source,
        "planned_beat_text": planned_beat_used,
        "custom_context": custom_context.strip(),
        "matched_hook_title": matched_hook_title or None,
        "suggested_tools": list(spec["suggested_tools"]),
        "mandate_text": mandate_text,
    }
    completed.append(beat_record)
    state["completed_beats"] = completed
    state["turns_since_last_beat"] = 0
    state["active_mandate"] = beat_record
    _SESSION_SCENARIO_DIRECTORS[session_id or "default"] = state
    return beat_record


@tool
def configure_scenario_director(
    session_id: str,
    campaign_premise: str = "",
    flow_mode: str = "hybrid",
    player_desires: list[str] | None = None,
    planned_beats: list[str] | None = None,
    event_weights: dict[str, float] | None = None,
    beat_probability_per_turn: float | None = None,
    beat_interval_turns: int = 3,
    seed_pack_starter: bool = False,
) -> dict[str, Any]:
    """Configure the session's Scenario Director (`flow_mode`, `campaign_premise`, `planned_beats`, and tunable `event_weights` probability dials).

    - `flow_mode`: `"hybrid"` (plot milestones + probability dials), `"scenario_driven"` (non-railroaded plot adaptation), or `"free_form"` (pure probability dials).
    - `event_weights`: Dict mapping any of the 12 TTRPG event slugs (or shorthand pillar names like `"combat"`, `"drama"`, `"intrigue"`, `"thought_provoking_discussion"`) to weights `>= 0.0`. Setting a weight to `0` disables that event completely; higher numbers increase its relative probability.
    - `seed_pack_starter`: When `True`, populates `campaign_premise` and `planned_beats` from the active campaign pack's `starter_scenario` whenever explicit overrides are omitted.
    """
    current = get_session_scenario_director(session_id)
    starter_premise = ""
    starter_beats: list[str] = []
    if seed_pack_starter:
        from satya_ml.campaign_config import get_pack_starter_scenario

        starter = get_pack_starter_scenario()
        starter_premise = str(starter.get("campaign_premise") or "")
        starter_beats = list(starter.get("planned_beats") or [])

    base_weights = (
        dict(current.get("event_weights") or default_event_weights())
        if event_weights is None and player_desires is None
        else default_event_weights()
    )
    if isinstance(event_weights, dict):
        for k, v in event_weights.items():
            slug = resolve_task_slug(str(k))
            if slug is not None:
                try:
                    base_weights[slug] = max(0.0, float(v))
                except (TypeError, ValueError):
                    pass
    prob_gate_active = beat_probability_per_turn is not None
    effective_prob = 0.35 if beat_probability_per_turn is None else float(beat_probability_per_turn)
    custom_premise_only = bool(campaign_premise.strip()) and not seed_pack_starter and planned_beats is None
    resolved_premise = campaign_premise or starter_premise or current.get("campaign_premise", "")
    if planned_beats is not None:
        resolved_beats = list(planned_beats)
    elif seed_pack_starter and starter_beats:
        resolved_beats = list(starter_beats)
    elif custom_premise_only:
        resolved_beats = []
    else:
        resolved_beats = list(current.get("planned_beats", []))
    auto_seeded = (
        False
        if (custom_premise_only or seed_pack_starter or planned_beats is not None)
        else bool(current.get("auto_seeded_starter", False))
    )
    updated = set_session_scenario_director(
        session_id,
        {
            "flow_mode": flow_mode,
            "campaign_premise": resolved_premise,
            "player_desires": player_desires if player_desires is not None else current.get("player_desires", []),
            "planned_beats": resolved_beats,
            "auto_seeded_starter": auto_seeded,
            "event_weights": base_weights,
            "beat_probability_per_turn": effective_prob,
            "probability_gate_active": prob_gate_active,
            "beat_interval_turns": beat_interval_turns,
        },
    )
    return {
        "status": "configured",
        "session_id": session_id,
        "flow_mode": updated["flow_mode"],
        "campaign_premise": updated["campaign_premise"],
        "planned_beats": updated["planned_beats"],
        "beat_probability_per_turn": updated["beat_probability_per_turn"],
        "beat_interval_turns": updated["beat_interval_turns"],
        "event_weights": updated["event_weights"],
        "normalized_probabilities": updated["normalized_probabilities"],
    }


@tool
def trigger_scenario_beat(
    session_id: str,
    task_slug: str = "auto",
    custom_context: str = "",
) -> dict[str, Any]:
    """Immediately select and trigger a proactive Scenario Director beat (`task_slug="auto"` or one of the 12 TTRPG event sub-routines) to drive the scenario forward."""
    beat = select_next_scenario_beat(
        session_id=session_id,
        task_slug=task_slug,
        custom_context=custom_context,
    )
    return {
        "status": "triggered",
        "session_id": session_id,
        **beat,
    }
