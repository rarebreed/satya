"""Lazy tool registry and discovery meta-tool (`load_gm_tools`) for the Satya AI GM.

Partitions the 47 built-in AI GM tools (plus any external MCP tools) into a small
always-on Core set (~12 tools) and 6 deferred domain packs so normal turns bind
~13 JSON schemas instead of all 47, with automatic state-based pre-activation for
active combat (`combat` pack) and OOC/rules queries (7 read-only tools) plus
turn-end expiry of dynamically loaded packs.
"""

from __future__ import annotations

import re
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from types import MappingProxyType
from typing import Any, Final, Literal, NotRequired, TypedDict

from satya_ml.ai_gm.tool_spec import (
    DeferredPackId,
    ensure_all_gm_tool_modules_imported,
    get_core_tool_names,
    get_ooc_readonly_tool_names,
    get_pack_tool_names,
    satya_gm_tool,
)


LOAD_GM_TOOLS_NAME: Final[Literal["load_gm_tools"]] = "load_gm_tools"
MAX_TOOL_LOAD_ROUNDS_PER_TURN: Final[int] = 2
LAZY_TOOL_MIN_THRESHOLD: Final[int] = 12


@dataclass(frozen=True, slots=True)
class ToolPackSpec:
    """Metadata and matching keywords for a deferred AI GM tool pack."""

    pack_id: str
    summary: str
    tool_names: tuple[str, ...]
    keywords: tuple[str, ...] = ()
    campaign_packs: tuple[str, ...] | None = None


@dataclass(frozen=True, slots=True)
class _PackDescriptor:
    summary: str
    keywords: tuple[str, ...]
    campaign_packs: tuple[str, ...] | None = None


class LoadedToolSummary(TypedDict):
    """Short name + description card returned for each activated deferred tool."""

    name: str
    summary: str


class ToolActivationResult(TypedDict):
    """Structured response returned by `GmToolRegistry.activate_tools` and `load_gm_tools`."""

    status: Literal["ok", "no_match"]
    loaded_packs: list[str]
    loaded_tool_names: list[str]
    loaded_tools: list[LoadedToolSummary]
    unmatched: list[str]
    available_packs: list[str]
    note: str
    pack_descriptions: NotRequired[dict[str, str]]


_PACK_DESCRIPTORS: Final[Mapping[DeferredPackId, _PackDescriptor]] = MappingProxyType(
    {
        "combat": _PackDescriptor(
            summary="Count-Up Tick combat tracker, token movement, ballistic shot LoS/cover, and reachable movement zones.",
            keywords=(
                "combat",
                "tick",
                "initiative",
                "attack",
                "shot",
                "ballistic",
                "firearm",
                "shoot",
                "move",
                "token",
                "reach",
                "movement",
                "range",
                "cover",
                "los",
                "action mode",
                "stance",
            ),
        ),
        "tactical_maps": _PackDescriptor(
            summary="Generate, continue, edit, switch floors, place clue pins, or inspect 2D/3D tactical battlemaps.",
            keywords=(
                "map",
                "battlemap",
                "floor",
                "stairs",
                "elevator",
                "door",
                "wall",
                "prop",
                "clue",
                "pin",
                "vision",
                "inspect",
                "terrain",
                "sector",
                "room",
                "zone",
                "layout",
            ),
        ),
        "social_and_psyche": _PackDescriptor(
            summary="9-axis Psyche empathy reads, social combat arguments, dissonance/conviction surges, NPC morale triggers, and on-the-fly NPC generation.",
            keywords=(
                "social",
                "empathy",
                "psyche",
                "argument",
                "persuade",
                "intimidate",
                "dissonance",
                "conviction",
                "surge",
                "morale",
                "trigger",
                "npc",
                "create npc",
                "generate npc",
                "sincerity",
                "trust",
            ),
        ),
        "world_and_director": _PackDescriptor(
            summary="Operational Heat & faction progress clocks, Scenario Director pacing/weights/beats, and explicit multi-channel narration.",
            keywords=(
                "clock",
                "faction",
                "heat",
                "jurisdiction",
                "alert",
                "director",
                "beat",
                "pacing",
                "scenario",
                "weights",
                "narrate",
                "broadcast",
            ),
        ),
        "tech_and_tradecraft": _PackDescriptor(
            summary="Stealth/RF hygiene/forensics, contacts/cover/safehouses, explosives/vehicles/hazards, EW/drones/engineering, audit proofs, tradecraft comms, and hybrid engine/rules RAG.",
            keywords=(
                "stealth",
                "surveillance",
                "forensics",
                "rf",
                "hack",
                "ew",
                "ecm",
                "eccm",
                "drone",
                "robot",
                "engineering",
                "explosive",
                "blast",
                "grenade",
                "vehicle",
                "hazard",
                "radiation",
                "contact",
                "cover",
                "safehouse",
                "audit",
                "proof",
                "tradecraft",
                "dead drop",
                "comms",
                "rag",
                "engine",
            ),
        ),
        "awakened_campaign": _PackDescriptor(
            summary="Awakened 2035 AI 2.0 synthetic substrates/embodiment, L0/L1 cage & hunter-AI interventions, proxy telemetry, conspiracy arcs, polycrisis grid strain, and Part V political economy.",
            keywords=(
                "awakened",
                "synthetic",
                "substrate",
                "shard",
                "embodiment",
                "cage",
                "l0",
                "l1",
                "hunter",
                "blue lotus",
                "manjusri",
                "proxy",
                "telemetry",
                "conspiracy",
                "pillar",
                "satyang",
                "arc",
                "polycrisis",
                "grid",
                "blackout",
                "brownout",
                "clamping",
                "economy",
            ),
            campaign_packs=("awakened",),
        ),
    }
)

ensure_all_gm_tool_modules_imported()

CORE_GM_TOOL_NAMES: Final[tuple[str, ...]] = get_core_tool_names()
OOC_READONLY_TOOL_NAMES: Final[tuple[str, ...]] = get_ooc_readonly_tool_names()
DEFERRED_TOOL_PACKS: Final[Mapping[DeferredPackId, ToolPackSpec]] = MappingProxyType(
    {
        pack_id: ToolPackSpec(
            pack_id=pack_id,
            summary=desc.summary,
            tool_names=get_pack_tool_names(pack_id),
            keywords=desc.keywords,
            campaign_packs=desc.campaign_packs,
        )
        for pack_id, desc in _PACK_DESCRIPTORS.items()
    }
)

_BUILTIN_DEFERRED_TOOL_NAMES: Final[frozenset[str]] = frozenset(
    name for pack in DEFERRED_TOOL_PACKS.values() for name in pack.tool_names
)
_CORE_TOOL_NAME_SET: Final[frozenset[str]] = frozenset(CORE_GM_TOOL_NAMES)
_TOKEN_RE: Final[re.Pattern[str]] = re.compile(r"[a-z0-9_]+")

# Intent-driven tool pre-activation rules for authoring/mutation requests that may arrive
# on `OUT_OF_CHARACTER` (or before the model discovers deferred packs), without matching
# read-only questions such as "How many NPCs do you have?" or "Show me the list of NPCs".
_TURN_INTENT_PREACTIVATION_RULES: Final[tuple[tuple[re.Pattern[str], tuple[str, ...]], ...]] = (
    (
        re.compile(
            r"\b(?:add|create|generate|make|roll\s+up|build|register|spawn|introduce|draft|new)\b[^.!?\n]{0,45}\b(?:npcs?|non[- ]player\s+characters?)\b",
            re.IGNORECASE,
        ),
        ("generate_npc_from_description",),
    ),
    (
        re.compile(
            r"\b(?:npcs?|non[- ]player\s+characters?)\b[^.!?\n]{0,25}\b(?:add|create|generate|roll\s+up|register|spawn|draft)\b",
            re.IGNORECASE,
        ),
        ("generate_npc_from_description",),
    ),
    (
        re.compile(
            r"\b(?:update|change|set|modify|edit)\b[^.!?\n]{0,35}\b(?:npc|disposition|immediate\s+agenda|combat\s+stance|fatigue|wound\s+shock)\b",
            re.IGNORECASE,
        ),
        ("update_npc_state",),
    ),
    (
        re.compile(
            r"\b(?:save|record|remember|note|add|store|log|pin)\b[^.!?\n]{0,35}\b(?:gm\s+memory|memory\s+note|session\s+note|gm\s+note|campaign\s+note|session\s+records?)\b",
            re.IGNORECASE,
        ),
        ("upsert_gm_memory",),
    ),
    (
        re.compile(
            r"\b(?:load|import|activate|bring\s+in|pull\s+in|add)\b[^.!?\n]{0,45}\b(?:campaign|starter|universal|pack|awakened)\s+npcs?\b"
            r"|\bload\b[^.!?\n]{0,40}\b(?:bandajara|alex\s+wen|sri\s+ramana|marcus\s+vance|elena\s+rostova|chloe\s+chen|cliff\s+reed|starter|campaign)\b",
            re.IGNORECASE,
        ),
        ("load_campaign_npc",),
    ),
    (
        re.compile(
            r"\b(?:delete|remove|erase|clear|purge)\b[^.!?\n]{0,35}\b(?:gm\s+memory|memory\s+note|session\s+note|gm\s+note|gm\s+doc|document)\b",
            re.IGNORECASE,
        ),
        ("delete_gm_memory",),
    ),
    (
        re.compile(
            r"\b(?:generate|create|make|build|draw|render)\b[^.!?\n]{0,30}\b(?:tactical\s+map|battlemap|battle\s+map|map)\b",
            re.IGNORECASE,
        ),
        ("generate_tactical_map",),
    ),
)


def _tool_name(tool_obj: Any) -> str:
    return str(getattr(tool_obj, "name", "") or getattr(tool_obj, "__name__", "")).strip()


def _tool_summary(tool_obj: Any) -> str:
    raw = str(
        getattr(tool_obj, "description", None)
        or getattr(tool_obj, "__doc__", None)
        or ""
    ).strip()
    if not raw:
        return ""
    first_line = next((ln.strip() for ln in raw.splitlines() if ln.strip()), "")
    if len(first_line) > 160:
        return first_line[:157].rstrip() + "..."
    return first_line


def _resolve_campaign_pack(campaign_pack: str | None) -> str:
    if campaign_pack is not None:
        return campaign_pack.strip().lower()
    try:
        from satya_ml.campaign_config import get_active_campaign_pack_name

        return get_active_campaign_pack_name().strip().lower()
    except Exception:
        return "awakened"


def _parse_str_list(raw: Sequence[str] | str | None) -> list[str]:
    if raw is None:
        return []
    items: list[str] = []
    if isinstance(raw, str):
        candidates = [raw]
    else:
        candidates = [str(x) for x in raw if x is not None]
    for cand in candidates:
        for part in re.split(r"[\s,]+", cand.strip()):
            cleaned = part.strip().lower()
            if cleaned and cleaned not in items:
                items.append(cleaned)
    return items


def _tokenize_query(text: str) -> list[str]:
    tokens: list[str] = []
    for raw_tok in _TOKEN_RE.findall(text.lower()):
        for part in [raw_tok, *raw_tok.split("_")]:
            if len(part) >= 2 and part not in tokens:
                tokens.append(part)
    return tokens


def _score_tool_match(tool_name: str, summary: str, q_lower: str, q_tokens: Sequence[str]) -> int:
    name_tokens = set(_tokenize_query(tool_name))
    doc_tokens = set(_tokenize_query(summary))
    exact_bonus = 8 if tool_name.lower() in q_lower else 0
    token_score = sum(
        4 if tok in name_tokens else (2 if tok in doc_tokens else 0)
        for tok in q_tokens
    )
    return exact_bonus + token_score


def _score_pack_match(
    spec: ToolPackSpec,
    tool_map: Mapping[str, Any],
    q_lower: str,
    q_tokens: Sequence[str],
) -> tuple[int, list[tuple[int, str]]]:
    pack_id_tokens = set(_tokenize_query(spec.pack_id))
    summary_tokens = set(_tokenize_query(spec.summary))
    score = 8 if spec.pack_id.lower() in q_lower else 0
    score += sum(
        (5 if kw.lower() in q_lower else 0) if " " in kw else (4 if kw.lower() in q_tokens else 0)
        for kw in spec.keywords
    )
    score += sum(
        6 if tok in pack_id_tokens else (2 if tok in summary_tokens else 0)
        for tok in q_tokens
    )
    per_tool = [
        (t_score, t_name)
        for t_name in spec.tool_names
        if t_name in tool_map
        and (t_score := _score_tool_match(t_name, _tool_summary(tool_map[t_name]), q_lower, q_tokens)) > 0
    ]
    score += sum(min(t_score, 6) for t_score, _ in per_tool)
    return score, per_tool


@satya_gm_tool(pack="meta", tier="TIER_1_SATYA_CORE", session_scoped=False)
def load_gm_tools(
    packs: list[str] | str = "",
    tool_names: list[str] | str = "",
    query: str = "",
) -> ToolActivationResult:
    """Load deferred AI GM domain tool packs or search tools by keyword for the current turn.

    Pass `packs` (e.g. `["combat"]`, `["tactical_maps"]`), exact `tool_names`, or a keyword `query`.
    """
    from satya_ml.ai_gm.tools import get_builtin_gm_tools

    registry = GmToolRegistry.from_tools(get_builtin_gm_tools())
    return registry.activate_tools(packs=packs, tool_names=tool_names, query=query)


class GmToolRegistry:
    """Turn-scoped lazy tool resolver for built-in AI GM tools and external MCP servers."""

    def __init__(
        self,
        tool_map: dict[str, Any],
        packs: dict[str, ToolPackSpec],
    ) -> None:
        self.tool_map = tool_map
        self.packs = packs
        self._tool_to_pack: dict[str, str] = {}
        for pack_id, spec in packs.items():
            for t_name in spec.tool_names:
                self._tool_to_pack.setdefault(t_name, pack_id)

    @classmethod
    def from_tools(cls, tools: Sequence[Any]) -> GmToolRegistry:
        """Index domain tools and build built-in + external MCP deferred packs."""
        tool_map: dict[str, Any] = {}
        for t in tools:
            name = _tool_name(t)
            if not name or name == LOAD_GM_TOOLS_NAME:
                continue
            tool_map[name] = t

        packs: dict[str, ToolPackSpec] = {}
        for pack_id, spec in DEFERRED_TOOL_PACKS.items():
            present = tuple(name for name in spec.tool_names if name in tool_map)
            if present:
                packs[pack_id] = ToolPackSpec(
                    pack_id=spec.pack_id,
                    summary=spec.summary,
                    tool_names=present,
                    keywords=spec.keywords,
                    campaign_packs=spec.campaign_packs,
                )

        external_buckets: dict[str, list[str]] = {}
        for name, tool_obj in tool_map.items():
            if name in _CORE_TOOL_NAME_SET or name in _BUILTIN_DEFERRED_TOOL_NAMES:
                continue
            meta = getattr(tool_obj, "metadata", None)
            srv = meta.get("mcp_server_id") if isinstance(meta, dict) else None
            if isinstance(srv, str) and srv.strip():
                bucket_id = f"mcp_{srv.strip().lower()}"
            else:
                bucket_id = "external_mcp"
            external_buckets.setdefault(bucket_id, []).append(name)

        for bucket_id, names in external_buckets.items():
            kw_set: list[str] = []
            for kw in _tokenize_query(bucket_id.removeprefix("mcp_")):
                if kw not in kw_set:
                    kw_set.append(kw)
            for t_name in names:
                for kw in _tokenize_query(t_name + " " + _tool_summary(tool_map[t_name])):
                    if kw not in kw_set:
                        kw_set.append(kw)
            preview = ", ".join(names[:4]) + ("..." if len(names) > 4 else "")
            if bucket_id.startswith("mcp_"):
                srv_label = bucket_id.removeprefix("mcp_")
                summary = f"External MCP server '{srv_label}' tools ({preview})."
            else:
                summary = f"External MCP tools ({preview})."
            packs[bucket_id] = ToolPackSpec(
                pack_id=bucket_id,
                summary=summary,
                tool_names=tuple(names),
                keywords=tuple(kw_set),
                campaign_packs=None,
            )

        return cls(tool_map=tool_map, packs=packs)

    def is_deferred_tool(self, tool_name: str) -> bool:
        """Return True if `tool_name` is a registered tool outside `CORE_GM_TOOL_NAMES`."""
        return tool_name in self.tool_map and tool_name not in _CORE_TOOL_NAME_SET

    def visible_deferred_packs(self, campaign_pack: str | None = None) -> list[ToolPackSpec]:
        """Return deferred tool packs available for the active `campaign_pack`."""
        norm_campaign = _resolve_campaign_pack(campaign_pack)
        visible: list[ToolPackSpec] = []
        for spec in self.packs.values():
            if spec.campaign_packs is not None and norm_campaign not in spec.campaign_packs:
                continue
            visible.append(spec)
        return visible

    def format_deferred_packs_prompt(self, campaign_pack: str | None = None) -> str:
        """Render a compact index of visible deferred tool packs without parameter schemas."""
        visible = self.visible_deferred_packs(campaign_pack=campaign_pack)
        if not visible:
            return ""
        lines = [
            "[Deferred Tool Packs — Call `load_gm_tools(packs=[...])` or `load_gm_tools(query=\"...\")` to load specialized tools for this turn]:"
        ]
        for spec in visible:
            extra = " (Auto-loaded while combat is active.)" if spec.pack_id == "combat" else ""
            lines.append(f"- `{spec.pack_id}`: {spec.summary}{extra}")
        return "\n".join(lines)

    def _visible_deferred_tool_names(self, campaign_pack: str | None = None) -> set[str]:
        return {
            t_name
            for spec in self.visible_deferred_packs(campaign_pack=campaign_pack)
            for t_name in spec.tool_names
        }

    def infer_preactivated_tool_names(
        self,
        text: str | None,
        *,
        campaign_pack: str | None = None,
    ) -> tuple[str, ...]:
        """Return tool names whose authoring/action intent matches `text` and are visible for `campaign_pack`."""
        raw = str(text or "").strip()
        if not raw:
            return ()
        visible = _CORE_TOOL_NAME_SET | self._visible_deferred_tool_names(campaign_pack)
        inferred: list[str] = []
        for pattern, tool_names in _TURN_INTENT_PREACTIVATION_RULES:
            if pattern.search(raw):
                for t_name in tool_names:
                    if t_name in self.tool_map and t_name in visible and t_name not in inferred:
                        inferred.append(t_name)
        return tuple(inferred)

    def resolve_bound_tools(
        self,
        *,
        is_ooc: bool = False,
        combat_active: bool = False,
        loaded_packs: Sequence[str] = (),
        loaded_names: Sequence[str] = (),
        preactivated_names: Sequence[str] = (),
        campaign_pack: str | None = None,
        include_loader: bool = True,
    ) -> tuple[list[Any], list[str]]:
        """Resolve the deduplicated tool objects and pack labels to bind for one supervisor call."""
        visible_map = {spec.pack_id: spec for spec in self.visible_deferred_packs(campaign_pack)}
        visible_names = _CORE_TOOL_NAME_SET | self._visible_deferred_tool_names(campaign_pack)
        bound: list[Any] = []
        seen_names: set[str] = set()
        labels: list[str] = []

        def _add_tool(name: str) -> None:
            if name in seen_names or name not in self.tool_map:
                return
            seen_names.add(name)
            bound.append(self.tool_map[name])

        if is_ooc:
            labels.append("ooc_readonly")
            for name in OOC_READONLY_TOOL_NAMES:
                _add_tool(name)
        else:
            labels.append("core")
            for name in CORE_GM_TOOL_NAMES:
                _add_tool(name)

        if combat_active and not is_ooc and "combat" in visible_map:
            labels.append("combat")
            for name in visible_map["combat"].tool_names:
                _add_tool(name)

        for raw_pack in loaded_packs:
            p_id = str(raw_pack or "").strip().lower()
            if p_id in visible_map:
                if p_id not in labels:
                    labels.append(p_id)
                for name in visible_map[p_id].tool_names:
                    _add_tool(name)

        extra_added = False
        for raw_name in loaded_names:
            t_name = str(raw_name or "").strip()
            if t_name in self.tool_map and t_name not in seen_names:
                _add_tool(t_name)
                extra_added = True
        if extra_added and "custom" not in labels:
            labels.append("custom")

        intent_added = False
        for raw_name in preactivated_names:
            t_name = str(raw_name or "").strip()
            if t_name in self.tool_map and t_name in visible_names and t_name not in seen_names:
                _add_tool(t_name)
                intent_added = True
        if intent_added and "intent" not in labels:
            labels.append("intent")

        if include_loader:
            visible_deferred = self._visible_deferred_tool_names(campaign_pack)
            if any(name not in seen_names for name in visible_deferred):
                bound.append(load_gm_tools)

        return bound, labels

    def activate_tools(
        self,
        *,
        packs: Sequence[str] | str = "",
        tool_names: Sequence[str] | str = "",
        query: str = "",
        campaign_pack: str | None = None,
    ) -> ToolActivationResult:
        """Resolve requested packs, tool names, or keyword query into turn-active tools."""
        visible_specs = self.visible_deferred_packs(campaign_pack=campaign_pack)
        visible_map = {spec.pack_id.lower(): spec for spec in visible_specs}
        visible_tool_names = set(_CORE_TOOL_NAME_SET) | {
            t_name for spec in visible_specs for t_name in spec.tool_names
        }
        lower_tool_lookup = {
            name.lower(): name for name in self.tool_map if name in visible_tool_names
        }

        loaded_packs: list[str] = []
        loaded_tool_names: list[str] = []
        unmatched: list[str] = []

        def _add_pack(spec: ToolPackSpec) -> None:
            if spec.pack_id not in loaded_packs:
                loaded_packs.append(spec.pack_id)
            for t_name in spec.tool_names:
                if t_name in self.tool_map and t_name not in loaded_tool_names:
                    loaded_tool_names.append(t_name)

        def _add_tool_name(name: str) -> None:
            if name in self.tool_map and name not in loaded_tool_names:
                loaded_tool_names.append(name)

        for item in _parse_str_list(packs):
            if item in visible_map:
                _add_pack(visible_map[item])
            elif item in lower_tool_lookup:
                _add_tool_name(lower_tool_lookup[item])
            else:
                unmatched.append(item)

        for item in _parse_str_list(tool_names):
            if item in lower_tool_lookup:
                _add_tool_name(lower_tool_lookup[item])
            elif item in visible_map:
                _add_pack(visible_map[item])
            else:
                unmatched.append(item)

        q_clean = str(query or "").strip()
        if q_clean:
            q_lower = q_clean.lower()
            q_tokens = _tokenize_query(q_clean)
            scored_packs = [
                (score, spec, per_tool)
                for spec in visible_specs
                for score, per_tool in (_score_pack_match(spec, self.tool_map, q_lower, q_tokens),)
            ]
            pack_scores = sorted(
                ((score, spec) for score, spec, _ in scored_packs if score > 0),
                key=lambda pair: (-pair[0], pair[1].pack_id),
            )
            if pack_scores:
                top_score, top_spec = pack_scores[0]
                _add_pack(top_spec)
                for sec_score, sec_spec in pack_scores[1:2]:
                    if sec_score >= max(4, int(top_score * 0.6)):
                        _add_pack(sec_spec)
            else:
                tool_scores = sorted(
                    (item for _, _, per_tool in scored_packs for item in per_tool),
                    key=lambda pair: (-pair[0], pair[1]),
                )
                for _, t_name in tool_scores[:6]:
                    _add_tool_name(t_name)

        available_packs = [spec.pack_id for spec in visible_specs]
        loaded_tools: list[LoadedToolSummary] = [
            LoadedToolSummary(name=name, summary=_tool_summary(self.tool_map[name]))
            for name in loaded_tool_names
            if name in self.tool_map
        ]

        if loaded_tool_names:
            return ToolActivationResult(
                status="ok",
                loaded_packs=loaded_packs,
                loaded_tool_names=loaded_tool_names,
                loaded_tools=loaded_tools,
                unmatched=unmatched,
                available_packs=available_packs,
                note=(
                    f"Loaded {len(loaded_tool_names)} tool(s) for this turn. "
                    "You may now call them directly."
                ),
            )

        return ToolActivationResult(
            status="no_match",
            loaded_packs=[],
            loaded_tool_names=[],
            loaded_tools=[],
            unmatched=unmatched,
            available_packs=available_packs,
            pack_descriptions={spec.pack_id: spec.summary for spec in visible_specs},
            note=(
                "No deferred tools matched your request. Pass one of `available_packs` "
                "or try a broader keyword `query`."
            ),
        )
