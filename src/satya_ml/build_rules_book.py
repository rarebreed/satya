"""Static HTML Rules Book generator and Python/Marimo helper bridge for `docs/ttrpg/`.

Can use the compiled Rust `satya_py` PyO3 extension when installed, and includes a 100%
compatible pure-Python fallback so Marimo notebooks (`notebooks/satya_rules_book.py`) and
`python -m satya_ml.build_rules_book` work in any environment.
"""

from __future__ import annotations

import html
import json
import math
from pathlib import Path
import re
from typing import Any

from satya_ml.dice_probabilities import (
    compute_stats,
    paired_list_signed_margin_pmfs,
    using_satya_py_backend,
)

try:
    import satya_py as _satya_py  # type: ignore[import-not-found]
except ImportError:
    _satya_py = None

BOOK_CHAPTER_SPECS: list[dict[str, str]] = [
    {
        "chapter_id": "background",
        "part": "Part I: Philosophy & Design Vision",
        "title": "An RPG for a More Civilized Age",
        "relative_path": "design-notes/background.md",
        "description": "Origins of Satya, realism vs. escapism, and how Satya solves 17 structural flaws of traditional RPG systems.",
    },
    {
        "chapter_id": "satya_d20_rules",
        "part": "Part II: Core 2d20 Watershed Resolution & Probability",
        "title": "Satya d20 Core Ruleset Reference",
        "relative_path": "satya_d20_rules.md",
        "description": "Canonical Two-List Cascading (2d20) dice mechanic, TN_eff = 31 - C + Delta_ext, carry modes, signed outcome tiers, and the Count-Up Action Clock.",
    },
    {
        "chapter_id": "probability",
        "part": "Part II: Core 2d20 Watershed Resolution & Probability",
        "title": "Probability & The Cascading Dice Mechanic",
        "relative_path": "probability.md",
        "description": "Mathematical foundation, distribution topology (skewness, modality, tail behavior), scale invariance across K, and avoiding the Skilled Botch Paradox.",
    },
    {
        "chapter_id": "tasks",
        "part": "Part II: Core 2d20 Watershed Resolution & Probability",
        "title": "Task Resolution & Skill Check Mechanics",
        "relative_path": "tasks.md",
        "description": "Step-by-step task resolution pipeline, Gate Classifications (Class A / B / C), Tool Tiers, Complementary KNO -> PHY Skill Synergy, and worked examples.",
    },
    {
        "chapter_id": "characteristics",
        "part": "Part III: Characters, Attributes & Lifepaths",
        "title": "Character Characteristics & Attribute Architecture",
        "relative_path": "character/characteristics.md",
        "description": "Physical, Mental, Social, Sensory, and Psyche (EGO, SLF, OPN, BLG, EXT, CNF, AGG) characteristics on the 1-20 Controlling Scale plus derived metrics.",
    },
    {
        "chapter_id": "skill-design",
        "part": "Part III: Characters, Attributes & Lifepaths",
        "title": "Skill System & Lifepath Architecture",
        "relative_path": "skill-design.md",
        "description": "Talents vs. Physical and Knowledge skills, 8 hierarchical Skill Trees, Master Task Requirements Table, and chronological Lifepath creation.",
    },
    {
        "chapter_id": "social_interactions",
        "part": "Part IV: Psyche, Social Combat & Tactical Combat",
        "title": "Social Interactions, Psyche Mechanics & Social Combat",
        "relative_path": "social_interactions.md",
        "description": "Acting against Psyche (Friction Checks & Trait Drift), Conviction Surges, Combat Morale modifiers, and Social Combat (Cognitive Dissonance).",
    },
    {
        "chapter_id": "combat",
        "part": "Part IV: Psyche, Social Combat & Tactical Combat",
        "title": "Combat Architecture, Morale & Ranged Firefights",
        "relative_path": "combat/combat.md",
        "description": "Combat philosophy, anatomical trauma over Hit Points, Count-Up Action Clock, 5 Morale Conditions, the Envelope & Fill Tactical Pool, and Ranged Firefights.",
    },
    {
        "chapter_id": "melee_combat",
        "part": "Part IV: Psyche, Social Combat & Tactical Combat",
        "title": "Melee Combat, Close-Quarters Tactics & The Tactical Triad",
        "relative_path": "combat/melee_combat.md",
        "description": "Close-quarters anatomy, the Tactical Triad (Tempo, Control, Force), Guard & Seizing the Initiative, Root vs. throws, and Viktor vs. Elena walkthrough.",
    },
]


def _default_repo_root() -> Path:
    return Path(__file__).resolve().parent.parent.parent


def extract_markdown_headings(markdown_text: str) -> list[str]:
    """Extract top-level `## ` and `### ` headings outside fenced code blocks."""
    headings: list[str] = []
    in_code = False
    for line in markdown_text.splitlines():
        s = line.strip()
        if s.startswith("```"):
            in_code = not in_code
            continue
        if in_code:
            continue
        if s.startswith("## "):
            headings.append(s[3:].strip())
        elif s.startswith("### "):
            headings.append(s[4:].strip())
    return headings


def load_chapter_markdown_by_id(chapter_id: str, repo_root: Path | None = None) -> str:
    """Load the canonical Markdown for `chapter_id` from `docs/ttrpg/` (or `satya_py`)."""
    norm = chapter_id.strip().lower().replace("_", "-")
    if _satya_py is not None:
        try:
            doc = _satya_py.get_rules_chapter(chapter_id)
            if doc is not None:
                return str(doc.markdown)
        except Exception:
            pass

    root = repo_root or _default_repo_root()
    for spec in BOOK_CHAPTER_SPECS:
        if spec["chapter_id"].lower().replace("_", "-") == norm:
            path = root / "docs" / "ttrpg" / spec["relative_path"]
            if path.exists():
                return path.read_text(encoding="utf-8")
    return f"# Chapter `{chapter_id}` not found"


def search_rules_book_python(
    query: str,
    max_results: int = 5,
    repo_root: Path | None = None,
) -> list[dict[str, Any]]:
    """Search across all 9 canonical Satya Rules Book chapters and return ranked section dicts."""
    if _satya_py is not None:
        try:
            hits = _satya_py.search_rules_book(query, max_results)
            return [
                {
                    "chapter_id": h.chapter_id,
                    "chapter_title": h.chapter_title,
                    "relative_path": h.relative_path,
                    "section_heading": h.section_heading,
                    "snippet": h.snippet,
                    "score": int(h.score),
                }
                for h in hits
            ]
        except Exception:
            pass

    root = repo_root or _default_repo_root()
    q_clean = query.strip().lower()
    tokens = [t for t in re.split(r"[^a-z0-9_]+", q_clean) if len(t) >= 2]
    results: list[dict[str, Any]] = []

    for spec in BOOK_CHAPTER_SPECS:
        path = root / "docs" / "ttrpg" / spec["relative_path"]
        if not path.exists():
            continue
        md = path.read_text(encoding="utf-8")
        sections = md.split("\n## ")
        for idx, raw_sec in enumerate(sections):
            sec_text = raw_sec.strip() if idx == 0 else f"## {raw_sec}".strip()
            if not sec_text:
                continue
            first_line = sec_text.splitlines()[0].lstrip("#").strip() or spec["title"]
            body_low = sec_text.lower()
            head_low = f"{spec['title']} {first_line}".lower()
            score = 0
            if q_clean and q_clean in head_low:
                score += 25
            if q_clean and q_clean in body_low:
                score += 10
            for tok in tokens:
                score += head_low.count(tok) * 8 + body_low.count(tok) * 2
            if score > 0 or not tokens:
                snippet = sec_text[:700] + ("..." if len(sec_text) > 700 else "")
                results.append(
                    {
                        "chapter_id": spec["chapter_id"],
                        "chapter_title": spec["title"],
                        "relative_path": spec["relative_path"],
                        "section_heading": first_line,
                        "snippet": snippet,
                        "score": max(score, 1),
                    }
                )

    results.sort(key=lambda item: int(item["score"]), reverse=True)
    return results[: max(1, max_results)]


def evaluate_cascade_pairs_python(
    skill_level: int,
    control_c: int,
    internal_mod: int,
    external_mod: int,
    pairs: list[tuple[int, int]],
    carry_mode: str = "markov_step",
    botch_rule: str = "closest_miss",
) -> dict[str, Any]:
    """Evaluate a concrete sequence of `(die_a, die_b)` `2d20` pairs against a Satya task."""
    if _satya_py is not None:
        try:
            out = _satya_py.evaluate_cascade_roll(
                skill_level,
                control_c,
                internal_mod,
                external_mod,
                pairs,
                carry_mode,
                botch_rule,
            )
            return {
                "steps": [
                    {
                        "pair_index": int(s.pair_index),
                        "die_a": int(s.die_a),
                        "die_b": int(s.die_b),
                        "raw_sum": int(s.raw_sum),
                        "carry_in": int(s.carry_in),
                        "effective_y": int(s.effective_y),
                        "tn_eff": int(s.tn_eff),
                        "succeeded": bool(s.succeeded),
                        "step_botch": bool(s.step_botch),
                        "miss_distance": int(s.miss_distance),
                        "carry_out": int(s.carry_out),
                    }
                    for s in out.steps
                ],
                "total_successes": int(out.total_successes),
                "raw_botches": int(out.raw_botches),
                "effective_botches": int(out.effective_botches),
                "min_miss_tier": int(out.min_miss_tier),
                "signed_margin": int(out.signed_margin),
                "final_pair_y": int(out.final_pair_y),
                "outcome_kind": str(out.outcome_kind),
                "outcome_tier": str(out.outcome_tier),
            }
        except Exception:
            pass

    n_pairs = max(1, int(skill_level))
    tn_eff = 31 - int(control_c) + int(external_mod)
    delta_over = 6
    delta_under = max(0, 6 + int(internal_mod))
    carry = 0
    total_successes = 0
    raw_botches = 0
    min_miss_tier = 5
    steps: list[dict[str, Any]] = []
    final_y = 0

    for idx, (da, db) in enumerate(pairs[:n_pairs], start=1):
        raw_sum = int(da) + int(db)
        carry_in = carry
        yn = raw_sum + carry_in
        final_y = yn
        succeeded = yn > tn_eff
        step_botch = (yn < tn_eff) and (yn < 2)
        if succeeded:
            total_successes += 1
            miss_dist = 0
            min_miss_tier = 0
            carry_out = (
                delta_over
                if carry_mode == "markov_step"
                else (carry_in + delta_over if carry_mode == "cumulative_snowball" else yn - tn_eff)
            )
        elif yn < tn_eff:
            miss_dist = tn_eff - yn
            min_miss_tier = min(min_miss_tier, min(5, miss_dist // 4))
            if step_botch:
                raw_botches += 1
            carry_out = (
                -delta_under
                if carry_mode == "markov_step"
                else (carry_in - delta_under if carry_mode == "cumulative_snowball" else yn - tn_eff)
            )
        else:
            miss_dist = 0
            min_miss_tier = 0
            carry_out = 0 if carry_mode == "markov_step" else carry_in

        carry = carry_out
        steps.append(
            {
                "pair_index": idx,
                "die_a": int(da),
                "die_b": int(db),
                "raw_sum": raw_sum,
                "carry_in": carry_in,
                "effective_y": yn,
                "tn_eff": tn_eff,
                "succeeded": succeeded,
                "step_botch": step_botch,
                "miss_distance": miss_dist,
                "carry_out": carry_out,
            }
        )

    eff_botches = (
        min_miss_tier
        if (total_successes == 0 and botch_rule == "closest_miss")
        else (raw_botches if total_successes == 0 else 0)
    )
    signed_margin = total_successes if total_successes > 0 else -eff_botches
    if total_successes >= 1 and raw_botches == 0:
        outcome_kind = "SUCCESS"
        outcome_tier = "EXTRAORDINARY_SUCCESS" if total_successes >= 3 else "SUCCESS"
    elif total_successes >= 1 and raw_botches >= 1:
        outcome_kind = "SUCCESS_WITH_COMPLICATION"
        outcome_tier = "PARTIAL_SUCCESS"
    elif eff_botches == 0:
        outcome_kind = "CLEAN_FAILURE"
        outcome_tier = "FAILURE"
    else:
        outcome_kind = "BOTCH_FAILURE"
        outcome_tier = "CRITICAL_FAILURE"

    return {
        "steps": steps,
        "total_successes": total_successes,
        "raw_botches": raw_botches,
        "effective_botches": eff_botches,
        "min_miss_tier": min_miss_tier,
        "signed_margin": signed_margin,
        "final_pair_y": final_y,
        "outcome_kind": outcome_kind,
        "outcome_tier": outcome_tier,
    }


def compute_tactical_pool_python(
    spd: int = 12,
    for_stat: int = 12,
    dex: int = 13,
    kin: int = 13,
    sig: int = 11,
    ins: int = 12,
    itu: int = 12,
    dis: int = 12,
    fit: int = 12,
    mass_kg: float = 75.0,
    skill_c: int = 14,
    skill_n: int = 4,
    is_ranged: bool = False,
    fatigue_level: int = 0,
    wound_shock_index: int = 0,
    leg_wound_penalty: int = 0,
    arm_head_wound_penalty: int = 0,
    torso_shoulder_wound_penalty: int = 0,
    morale_state: str = "steady",
) -> dict[str, Any]:
    """Compute the Satya Combat Tactical Pool ('Envelope & Fill') parameters."""
    if _satya_py is not None:
        try:
            env = _satya_py.compute_tactical_pool(
                spd=spd,
                for_stat=for_stat,
                dex=dex,
                kin=kin,
                sig=sig,
                ins=ins,
                itu=itu,
                dis=dis,
                fit=fit,
                mass_kg=mass_kg,
                skill_c=skill_c,
                skill_n=skill_n,
                is_ranged=is_ranged,
                fatigue_level=fatigue_level,
                wound_shock_index=wound_shock_index,
                leg_wound_penalty=leg_wound_penalty,
                arm_head_wound_penalty=arm_head_wound_penalty,
                torso_shoulder_wound_penalty=torso_shoulder_wound_penalty,
                morale_state=morale_state,
            )
            return {
                "ooda_init": int(env.ooda_init),
                "combat_readiness_c": int(env.combat_readiness_c),
                "p_base": int(env.p_base),
                "p_eff": int(env.p_eff),
                "neutral_baseline_b": int(env.neutral_baseline_b),
                "tempo_cap": int(env.tempo_cap),
                "control_cap": int(env.control_cap),
                "force_cap": int(env.force_cap),
                "mastery_overflow": int(env.mastery_overflow),
                "power_pow": float(env.power_pow),
                "momentum_mass_factor_mmf": float(env.momentum_mass_factor_mmf),
                "fatigue_threshold_ft": int(env.fatigue_threshold_ft),
                "action_time_factor": float(env.action_time_factor),
                "morale_state": str(env.morale_state),
                "min_force_required": int(env.min_force_required),
                "max_control_allowed": int(env.max_control_allowed),
                "max_force_allowed": int(env.max_force_allowed),
                "force_must_not_exceed_control": bool(env.force_must_not_exceed_control),
            }
        except Exception:
            pass

    ooda = (spd + ins + itu) // 3
    c_combat = (skill_c + ooda) // 2
    p_base = c_combat + 3 * skill_n
    b_val = p_base // 3
    overflow = max(0, (skill_n - 3) // 2) if skill_n >= 5 else 0

    t_cap = max(0, ((spd + kin) // 2) + overflow - leg_wound_penalty)
    ctrl_stat = ((dex + sig) // 2) if is_ranged else ((dex + kin) // 2)
    c_cap = max(0, ctrl_stat + overflow - arm_head_wound_penalty)
    f_cap = max(0, ((for_stat + spd) // 2) + overflow - torso_shoulder_wound_penalty)

    m_norm = morale_state.strip().lower()
    morale_pool_mod = {"hesitant": -2, "shaken": -4, "resolute": 2}.get(m_norm, 0)
    p_eff = max(0, p_base - 2 * max(0, fatigue_level) - 3 * max(0, wound_shock_index) + morale_pool_mod)

    min_f = min(f_cap, math.ceil(p_eff / 2.0)) if m_norm == "enraged" else 0
    max_c = min(c_cap, b_val) if m_norm == "enraged" else c_cap
    max_f = min(f_cap, b_val) if m_norm == "shaken" else f_cap

    return {
        "ooda_init": ooda,
        "combat_readiness_c": c_combat,
        "p_base": p_base,
        "p_eff": p_eff,
        "neutral_baseline_b": b_val,
        "tempo_cap": t_cap,
        "control_cap": c_cap,
        "force_cap": f_cap,
        "mastery_overflow": overflow,
        "power_pow": (for_stat * spd) / 10.0,
        "momentum_mass_factor_mmf": (mass_kg / 70.0) * spd,
        "fatigue_threshold_ft": fit + (dis // 2),
        "action_time_factor": 1.0 + 0.10 * max(0, fatigue_level) + 0.25 * max(0, wound_shock_index),
        "morale_state": m_norm,
        "min_force_required": min_f,
        "max_control_allowed": max_c,
        "max_force_allowed": max_f,
        "force_must_not_exceed_control": m_norm == "hesitant",
    }


def evaluate_tactical_triad_python(
    envelope: dict[str, Any],
    base_weapon_tc: int,
    tempo: int,
    control: int,
    force: int,
) -> dict[str, Any]:
    """Validate and score a `(Tempo, Control, Force)` allocation against `envelope`."""
    violations: list[str] = []
    if tempo < 0 or control < 0 or force < 0:
        violations.append("Tempo, Control, and Force must be non-negative.")
    total = tempo + control + force
    if total > int(envelope["p_eff"]):
        violations.append(f"Total T+C+F ({total}) exceeds P_eff ({envelope['p_eff']}).")
    if tempo > int(envelope["tempo_cap"]):
        violations.append(f"Tempo ({tempo}) exceeds T_max ({envelope['tempo_cap']}).")
    if control > int(envelope["max_control_allowed"]):
        violations.append(f"Control ({control}) exceeds C_max ({envelope['max_control_allowed']}).")
    if force > int(envelope["max_force_allowed"]):
        violations.append(f"Force ({force}) exceeds F_max ({envelope['max_force_allowed']}).")
    if force < int(envelope["min_force_required"]):
        violations.append(f"Force ({force}) is below Enraged minimum ({envelope['min_force_required']}).")
    if envelope.get("force_must_not_exceed_control") and force > control:
        violations.append(f"Hesitant morale requires Force ({force}) <= Control ({control}).")

    b_val = max(1, int(envelope["neutral_baseline_b"]))
    raw_tc = max(3, int(base_weapon_tc) - ((tempo - b_val) // 2))
    eff_tc = max(3, int(round(raw_tc * float(envelope["action_time_factor"]))))
    strike_impact = float(envelope["power_pow"]) * (float(max(0, force)) / float(b_val))
    eff_root = float(max(0, force)) + float(envelope["momentum_mass_factor_mmf"])
    exertion = 2 if (force > b_val + 4 or tempo > b_val + 4) else (1 if (total == envelope["p_eff"] and total > 0) else 0)

    hi_t = tempo > b_val
    hi_c = control > b_val
    hi_f = force > b_val
    if hi_t and hi_c and not hi_f:
        arch = "Tempo + Control (TC): Probe, Feint & Evasive Counter"
    elif hi_t and hi_f and not hi_c:
        arch = "Tempo + Force (TF): Blitz, Charge & Snap Cleave"
    elif hi_c and hi_f and not hi_t:
        arch = "Control + Force (CF): Measured Execution & Joint Lock"
    elif hi_t and not hi_c and not hi_f:
        arch = "Pure Tempo (T): Disengage, Sprint & Panic Parry"
    elif hi_c and not hi_t and not hi_f:
        arch = "Pure Control (C): Total Guard & Aimed Precision"
    elif hi_f and not hi_t and not hi_c:
        arch = "Pure Force (F): Haymaker, Shield Bash & Break Grapple"
    else:
        arch = "Balanced Stance (Neutral Baseline)"

    return {
        "tempo": tempo,
        "control": control,
        "force": force,
        "valid": len(violations) == 0,
        "violations": violations,
        "effective_tc": eff_tc,
        "strike_impact": strike_impact,
        "effective_root": eff_root,
        "exertion_marks_gained": exertion,
        "archetype_label": arch,
    }


def render_markdown_to_html_python(markdown_text: str) -> str:
    """Render Satya Rules Book Markdown into safe HTML (using `satya_py` when available)."""
    if _satya_py is not None:
        try:
            return str(_satya_py.render_rules_markdown_to_html(markdown_text))
        except Exception:
            pass

    lines = markdown_text.splitlines()
    out: list[str] = ['<article class="satya-rules-article">']
    i = 0

    def _inline(text: str) -> str:
        esc = html.escape(text)
        esc = re.sub(r"`([^`]+)`", r'<code class="rules-inline-code">\1</code>', esc)
        esc = re.sub(r"\*\*([^*]+)\*\*", r"<strong>\1</strong>", esc)
        esc = re.sub(r"\*([^*]+)\*", r"<em>\1</em>", esc)
        esc = re.sub(
            r"\[([^\]]+)\]\(([^)]+)\)",
            r'<a href="\2" class="rules-link">\1</a>',
            esc,
        )
        return esc

    while i < len(lines):
        s = lines[i].strip()
        if not s:
            i += 1
            continue
        if s.startswith("```"):
            lang = s[3:].strip()
            i += 1
            buf: list[str] = []
            while i < len(lines) and not lines[i].strip().startswith("```"):
                buf.append(lines[i])
                i += 1
            if i < len(lines):
                i += 1
            if lang == "mermaid":
                out.append(f'<pre class="mermaid">{html.escape(chr(10).join(buf))}</pre>')
            else:
                out.append(
                    f'<pre class="rules-code-block"><code class="language-{html.escape(lang)}">'
                    f"{html.escape(chr(10).join(buf))}</code></pre>"
                )
            continue
        if s == "---":
            out.append('<hr class="rules-divider"/>')
            i += 1
            continue
        if s.startswith("#### "):
            out.append(f'<h4 class="rules-h4">{_inline(s[5:])}</h4>')
            i += 1
            continue
        if s.startswith("### "):
            out.append(f'<h3 class="rules-h3">{_inline(s[4:])}</h3>')
            i += 1
            continue
        if s.startswith("## "):
            out.append(f'<h2 class="rules-h2">{_inline(s[3:])}</h2>')
            i += 1
            continue
        if s.startswith("# "):
            out.append(f'<h1 class="rules-h1">{_inline(s[2:])}</h1>')
            i += 1
            continue
        if s.startswith("|") and s.endswith("|"):
            t_rows: list[str] = []
            while i < len(lines) and lines[i].strip().startswith("|") and lines[i].strip().endswith("|"):
                t_rows.append(lines[i].strip())
                i += 1
            out.append('<div class="rules-table-wrap"><table class="rules-table">')
            for r_idx, r_str in enumerate(t_rows):
                cells = [c.strip() for c in r_str.strip("|").split("|")]
                if all(c and all(ch in "-: " for ch in c) for c in cells):
                    continue
                if r_idx == 0:
                    out.append(
                        "<thead><tr>"
                        + "".join(f"<th>{_inline(c)}</th>" for c in cells)
                        + "</tr></thead><tbody>"
                    )
                else:
                    out.append("<tr>" + "".join(f"<td>{_inline(c)}</td>" for c in cells) + "</tr>")
            out.append("</tbody></table></div>")
            continue
        if s.startswith("- ") or s.startswith("* "):
            out.append('<ul class="rules-list">')
            while i < len(lines) and (lines[i].strip().startswith("- ") or lines[i].strip().startswith("* ")):
                out.append(f"<li>{_inline(lines[i].strip()[2:])}</li>")
                i += 1
            out.append("</ul>")
            continue
        out.append(f"<p>{_inline(s)}</p>")
        i += 1

    out.append("</article>")
    return "\n".join(out)


def _rewrite_chapter_links(rendered_html: str) -> str:
    """Rewrite relative `.md` links in rendered HTML so static book chapters link to `<chapter_id>.html`."""
    mapping = {
        "design-notes/background.md": "background.html",
        "../design-notes/background.md": "background.html",
        "satya_d20_rules.md": "satya_d20_rules.html",
        "../satya_d20_rules.md": "satya_d20_rules.html",
        "probability.md": "probability.html",
        "../probability.md": "probability.html",
        "tasks.md": "tasks.html",
        "../tasks.md": "tasks.html",
        "character/characteristics.md": "characteristics.html",
        "../character/characteristics.md": "characteristics.html",
        "characteristics.md": "characteristics.html",
        "skill-design.md": "skill-design.html",
        "../skill-design.md": "skill-design.html",
        "social_interactions.md": "social_interactions.html",
        "../social_interactions.md": "social_interactions.html",
        "combat/combat.md": "combat.html",
        "../combat/combat.md": "combat.html",
        "combat.md": "combat.html",
        "combat/melee_combat.md": "melee_combat.html",
        "../combat/melee_combat.md": "melee_combat.html",
        "melee_combat.md": "melee_combat.html",
    }
    out = rendered_html
    for src, dst in mapping.items():
        out = out.replace(f'href="{src}"', f'href="{dst}"')
    return out


_INTERACTIVE_WIDGET_HTML = """
<section class="book-interactive-widget">
  <h2>Interactive Satya 2d20 Watershed Cascade &amp; Tactical Pool Calculator</h2>
  <p class="widget-sub">
    Powered by the Satya Four-Factor Cascade formula
    (<code>TN_eff = 31 - C + &Delta;_ext</code>, <code>&delta;_over = +6</code>, <code>&delta;_under = -(6 + &Delta;_int)</code>)
    and the Combat Tactical Pool (<code>P_base = C_combat + 3N</code>).
  </p>
  <div class="widget-grid">
    <div class="widget-panel">
      <h3>1. 2d20 Watershed Cascade Simulator</h3>
      <div class="widget-controls">
        <label>Skill Level (N Pairs: 1..8)
          <input id="bk-n" type="number" min="1" max="8" value="4" onchange="satyaBookRecalc()"/>
        </label>
        <label>Controlling Characteristic (C: 1..20)
          <input id="bk-c" type="number" min="1" max="20" value="11" onchange="satyaBookRecalc()"/>
        </label>
        <label>Internal Mod (&Delta;_int: Fatigue/Shock)
          <input id="bk-dint" type="number" min="-2" max="10" value="2" onchange="satyaBookRecalc()"/>
        </label>
        <label>External Mod (&Delta;_ext: Difficulty/Tools)
          <input id="bk-dext" type="number" min="-6" max="12" value="2" onchange="satyaBookRecalc()"/>
        </label>
      </div>
      <div class="widget-presets">
        <button type="button" onclick="satyaBookLoadPreset(4,11,2,2,'14+11, 8+7, 15+13, 17+12')">Sarah's Lockpick</button>
        <button type="button" onclick="satyaBookLoadPreset(5,16,0,2,'12+10, 9+11, 14+8, 16+9, 11+12')">Elena's Probe</button>
        <button type="button" onclick="satyaBookLoadPreset(4,13,0,3,'15+11, 7+6, 13+10, 18+9')">Viktor's Cleave</button>
        <button type="button" onclick="satyaBookRandomRoll()">Roll Random N&times;2d20</button>
      </div>
      <label class="full-width">Rolled 2d20 Pairs (A+B, comma-separated):
        <input id="bk-pairs" type="text" value="14+11, 8+7, 15+13, 17+12" oninput="satyaBookRecalc()"/>
      </label>
      <div id="bk-cascade-readout" class="widget-readout"></div>
    </div>

    <div class="widget-panel">
      <h3>2. Combat Tactical Pool (Tempo &middot; Control &middot; Force)</h3>
      <div class="widget-controls">
        <label>Combat Readiness C_combat
          <input id="bk-ccombat" type="number" min="1" max="20" value="13" onchange="satyaBookRecalc()"/>
        </label>
        <label>Skill Level N
          <input id="bk-combat-n" type="number" min="1" max="8" value="4" onchange="satyaBookRecalc()"/>
        </label>
        <label>Tempo (T)
          <input id="bk-t" type="number" min="0" max="20" value="9" onchange="satyaBookRecalc()"/>
        </label>
        <label>Control (C)
          <input id="bk-ctrl" type="number" min="0" max="20" value="9" onchange="satyaBookRecalc()"/>
        </label>
        <label>Force (F)
          <input id="bk-f" type="number" min="0" max="20" value="7" onchange="satyaBookRecalc()"/>
        </label>
        <label>Base Weapon TC
          <input id="bk-wtc" type="number" min="4" max="16" value="8" onchange="satyaBookRecalc()"/>
        </label>
      </div>
      <div id="bk-tactical-readout" class="widget-readout"></div>
    </div>
  </div>
</section>
<script>
function satyaBookLoadPreset(n, c, dint, dext, pairs) {
  document.getElementById('bk-n').value = n;
  document.getElementById('bk-c').value = c;
  document.getElementById('bk-dint').value = dint;
  document.getElementById('bk-dext').value = dext;
  document.getElementById('bk-pairs').value = pairs;
  satyaBookRecalc();
}
function satyaBookRandomRoll() {
  const n = Math.max(1, Math.min(8, parseInt(document.getElementById('bk-n').value || '3', 10)));
  const rolls = [];
  for (let i = 0; i < n; i++) {
    const a = Math.floor(Math.random() * 20) + 1;
    const b = Math.floor(Math.random() * 20) + 1;
    rolls.push(a + '+' + b);
  }
  document.getElementById('bk-pairs').value = rolls.join(', ');
  satyaBookRecalc();
}
function satyaBookRecalc() {
  const n = Math.max(1, Math.min(8, parseInt(document.getElementById('bk-n').value || '4', 10)));
  const c = Math.max(1, Math.min(20, parseInt(document.getElementById('bk-c').value || '11', 10)));
  const dint = parseInt(document.getElementById('bk-dint').value || '0', 10);
  const dext = parseInt(document.getElementById('bk-dext').value || '0', 10);
  const tnEff = 31 - c + dext;
  const dOver = 6;
  const dUnder = Math.max(0, 6 + dint);
  const rawPairs = (document.getElementById('bk-pairs').value || '').split(',');
  let carry = 0, succ = 0, botch = 0, minMissTier = 5;
  let rows = [];
  for (let i = 0; i < n; i++) {
    const token = (rawPairs[i] || '11+10').trim().split('+');
    const a = parseInt(token[0] || '11', 10) || 11;
    const b = parseInt(token[1] || '10', 10) || 10;
    const raw = a + b;
    const yn = raw + carry;
    let status = 'Miss';
    let nextCarry = 0;
    if (yn > tnEff) {
      succ += 1;
      minMissTier = 0;
      status = '<strong style="color:#34d399">SUCCESS (+1)</strong>';
      nextCarry = dOver;
    } else if (yn < tnEff) {
      const dist = tnEff - yn;
      minMissTier = Math.min(minMissTier, Math.min(5, Math.floor(dist / 4)));
      if (yn < 2) {
        botch += 1;
        status = '<strong style="color:#fb7185">FLOOR BREACH (&lt;2)</strong>';
      }
      nextCarry = -dUnder;
    } else {
      minMissTier = 0;
      status = 'Exact Tie (0)';
      nextCarry = 0;
    }
    rows.push('<tr><td>#' + (i+1) + '</td><td>' + a + '+' + b + '=' + raw + '</td><td>' + (carry>=0?'+'+carry:carry) + '</td><td><strong>' + yn + '</strong></td><td>' + tnEff + '</td><td>' + status + '</td><td>' + (nextCarry>=0?'+'+nextCarry:nextCarry) + '</td></tr>');
    carry = nextCarry;
  }
  const effBotch = succ === 0 ? minMissTier : botch;
  const margin = succ > 0 ? '+' + succ : String(-effBotch);
  const tier = (succ >= 3 && botch === 0) ? 'EXTRAORDINARY_SUCCESS' : (succ >= 1 && botch === 0) ? 'SUCCESS' : (succ >= 1 && botch >= 1) ? 'PARTIAL_SUCCESS (Complication)' : (effBotch === 0 ? 'CLEAN_FAILURE' : 'CRITICAL_FAILURE');
  document.getElementById('bk-cascade-readout').innerHTML =
    '<div><strong>TN_eff = ' + tnEff + '</strong> | &delta;_over = +' + dOver + ' | &delta;_under = -' + dUnder + ' | <strong>Outcome: ' + tier + ' (Margin ' + margin + ')</strong></div>' +
    '<table class="rules-table" style="margin-top:8px"><thead><tr><th>Pair</th><th>Roll</th><th>Carry In</th><th>Y_n</th><th>TN_eff</th><th>Result</th><th>Carry Out</th></tr></thead><tbody>' + rows.join('') + '</tbody></table>';

  const cCombat = parseInt(document.getElementById('bk-ccombat').value || '13', 10);
  const combatN = parseInt(document.getElementById('bk-combat-n').value || '4', 10);
  const pBase = cCombat + 3 * combatN;
  const bVal = Math.floor(pBase / 3);
  const tVal = parseInt(document.getElementById('bk-t').value || '0', 10);
  const ctrlVal = parseInt(document.getElementById('bk-ctrl').value || '0', 10);
  const fVal = parseInt(document.getElementById('bk-f').value || '0', 10);
  const wTc = parseInt(document.getElementById('bk-wtc').value || '8', 10);
  const sumTcf = tVal + ctrlVal + fVal;
  const effTc = Math.max(3, wTc - Math.floor((tVal - bVal) / 2));
  const impactMult = (fVal / Math.max(1, bVal)).toFixed(2);
  const validBadge = sumTcf <= pBase
    ? '<span style="color:#34d399">VALID (' + sumTcf + ' / ' + pBase + ' pts)</span>'
    : '<span style="color:#fb7185">OVER POOL (' + sumTcf + ' &gt; ' + pBase + ' pts)</span>';
  document.getElementById('bk-tactical-readout').innerHTML =
    '<p><strong>Pool P_base = ' + pBase + '</strong> | Neutral Baseline <strong>B = ' + bVal + '</strong> | Allocation: ' + validBadge + '</p>' +
    '<ul><li><strong>Effective Action Cost (TC)</strong>: <code>' + effTc + ' ticks</code> (Base Weapon TC ' + wTc + ')</li>' +
    '<li><strong>Control Task Bonus</strong>: <code>' + (ctrlVal - bVal >= 0 ? '+' + (ctrlVal - bVal) : (ctrlVal - bVal)) + '</code> vs. Neutral Baseline</li>' +
    '<li><strong>Force Impact Multiplier</strong>: <code>' + impactMult + '&times; POW</code></li></ul>';
}
window.addEventListener('DOMContentLoaded', satyaBookRecalc);
</script>
"""


def _wrap_book_page_html(
    title: str,
    active_chapter_id: str,
    body_html: str,
    prev_spec: dict[str, str] | None = None,
    next_spec: dict[str, str] | None = None,
) -> str:
    nav_items: list[str] = [
        f'<a href="index.html" class="book-nav-link {"active" if active_chapter_id == "index" else ""}">'
        "0. Table of Contents &amp; Interactive Sandbox</a>"
    ]
    current_part = ""
    for idx, spec in enumerate(BOOK_CHAPTER_SPECS, start=1):
        if spec["part"] != current_part:
            current_part = spec["part"]
            nav_items.append(f'<div class="book-nav-part">{html.escape(current_part)}</div>')
        cls = "book-nav-link active" if spec["chapter_id"] == active_chapter_id else "book-nav-link"
        nav_items.append(
            f'<a href="{spec["chapter_id"]}.html" class="{cls}">'
            f"{idx}. {html.escape(spec['title'])}</a>"
        )

    footer_links: list[str] = []
    if prev_spec is not None:
        footer_links.append(
            f'<a class="book-pager-btn" href="{prev_spec["chapter_id"]}.html">&larr; Previous: {html.escape(prev_spec["title"])}</a>'
        )
    else:
        footer_links.append('<a class="book-pager-btn" href="index.html">&larr; Table of Contents</a>')
    if next_spec is not None:
        footer_links.append(
            f'<a class="book-pager-btn" href="{next_spec["chapter_id"]}.html">Next: {html.escape(next_spec["title"])} &rarr;</a>'
        )

    return f"""<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8"/>
  <meta name="viewport" content="width=device-width, initial-scale=1"/>
  <title>{html.escape(title)} — Satya TTRPG Rules Book</title>
  <style>
    :root {{
      --bg: #090d16;
      --panel: #111827;
      --border: #1f2937;
      --text: #e2e8f0;
      --muted: #94a3b8;
      --accent: #38bdf8;
      --emerald: #34d399;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      font-family: Inter, system-ui, -apple-system, sans-serif;
      background: var(--bg);
      color: var(--text);
      display: grid;
      grid-template-columns: 300px 1fr;
      min-height: 100vh;
    }}
    aside.book-sidebar {{
      background: var(--panel);
      border-right: 1px solid var(--border);
      padding: 20px 16px;
      position: sticky;
      top: 0;
      height: 100vh;
      overflow-y: auto;
    }}
    .book-brand {{
      font-size: 1.15rem;
      font-weight: 700;
      color: var(--accent);
      margin-bottom: 4px;
    }}
    .book-sub {{
      font-size: 0.8rem;
      color: var(--muted);
      margin-bottom: 16px;
    }}
    .book-nav-part {{
      font-size: 0.72rem;
      text-transform: uppercase;
      letter-spacing: 0.06em;
      color: var(--muted);
      margin: 14px 0 6px;
      font-weight: 600;
    }}
    .book-nav-link {{
      display: block;
      padding: 7px 10px;
      border-radius: 6px;
      color: var(--text);
      text-decoration: none;
      font-size: 0.88rem;
      margin-bottom: 3px;
    }}
    .book-nav-link:hover {{ background: rgba(56, 189, 248, 0.12); }}
    .book-nav-link.active {{
      background: rgba(56, 189, 248, 0.2);
      color: var(--accent);
      font-weight: 600;
    }}
    main.book-main {{
      max-width: 980px;
      padding: 32px 44px 64px;
      line-height: 1.65;
    }}
    h1, h2, h3, h4 {{ color: #f8fafc; }}
    h1 {{ border-bottom: 1px solid var(--border); padding-bottom: 10px; }}
    h2 {{ margin-top: 2rem; border-bottom: 1px solid var(--border); padding-bottom: 6px; }}
    a {{ color: var(--accent); }}
    code, pre {{ font-family: "JetBrains Mono", Consolas, monospace; }}
    .rules-inline-code {{
      background: #1e293b;
      padding: 2px 6px;
      border-radius: 4px;
      font-size: 0.88em;
    }}
    .rules-code-block {{
      background: #0f172a;
      border: 1px solid var(--border);
      padding: 14px;
      border-radius: 8px;
      overflow-x: auto;
    }}
    .rules-table-wrap {{ overflow-x: auto; margin: 14px 0; }}
    .rules-table {{
      width: 100%;
      border-collapse: collapse;
      font-size: 0.9rem;
    }}
    .rules-table th, .rules-table td {{
      border: 1px solid var(--border);
      padding: 8px 10px;
      text-align: left;
    }}
    .rules-table th {{ background: #1e293b; color: var(--accent); }}
    .rules-alert, .rules-blockquote {{
      border-left: 4px solid var(--accent);
      background: rgba(30, 41, 59, 0.55);
      padding: 10px 16px;
      border-radius: 0 8px 8px 0;
      margin: 14px 0;
    }}
    .rules-alert-title {{ font-weight: 700; color: var(--accent); margin-bottom: 4px; }}
    .book-interactive-widget {{
      background: #111827;
      border: 1px solid #334155;
      border-radius: 10px;
      padding: 20px;
      margin: 24px 0;
    }}
    .widget-grid {{
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 18px;
    }}
    .widget-panel {{
      background: #0f172a;
      border: 1px solid var(--border);
      border-radius: 8px;
      padding: 14px;
    }}
    .widget-controls {{
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 8px;
      margin-bottom: 10px;
    }}
    .widget-controls label, label.full-width {{
      display: flex;
      flex-direction: column;
      font-size: 0.8rem;
      color: var(--muted);
      gap: 4px;
    }}
    .widget-controls input, label.full-width input {{
      background: #1e293b;
      border: 1px solid #334155;
      color: var(--text);
      padding: 6px 8px;
      border-radius: 4px;
    }}
    .widget-presets {{
      display: flex;
      flex-wrap: wrap;
      gap: 6px;
      margin: 8px 0;
    }}
    .widget-presets button {{
      background: #1e293b;
      border: 1px solid #38bdf8;
      color: var(--accent);
      padding: 5px 10px;
      border-radius: 5px;
      cursor: pointer;
      font-size: 0.8rem;
    }}
    .book-pager {{
      display: flex;
      justify-content: space-between;
      margin-top: 40px;
      padding-top: 16px;
      border-top: 1px solid var(--border);
    }}
    .book-pager-btn {{
      background: var(--panel);
      border: 1px solid var(--border);
      padding: 8px 14px;
      border-radius: 6px;
      text-decoration: none;
    }}
    @media (max-width: 900px) {{
      body {{ grid-template-columns: 1fr; }}
      aside.book-sidebar {{ position: static; height: auto; }}
      .widget-grid {{ grid-template-columns: 1fr; }}
    }}
  </style>
</head>
<body>
  <aside class="book-sidebar">
    <div class="book-brand">SATYA // RULES BOOK</div>
    <div class="book-sub">2d20 Watershed Cascade &amp; Tactical Triad</div>
    {"".join(nav_items)}
  </aside>
  <main class="book-main">
    {body_html}
    <div class="book-pager">
      {"".join(footer_links)}
    </div>
  </main>
</body>
</html>
"""


def build_static_rules_book(
    output_dir: Path | None = None,
    repo_root: Path | None = None,
) -> dict[str, Any]:
    """Compile all 9 `docs/ttrpg/` chapters into static HTML pages and `catalog.json` inside `docs/book/`."""
    root = repo_root or _default_repo_root()
    out_dir = output_dir or (root / "docs" / "book")
    out_dir.mkdir(parents=True, exist_ok=True)

    catalog_chapters: list[dict[str, Any]] = []
    generated_files: list[str] = []

    for idx, spec in enumerate(BOOK_CHAPTER_SPECS):
        cid = spec["chapter_id"]
        md = load_chapter_markdown_by_id(cid, repo_root=root)
        headings = extract_markdown_headings(md)
        rendered = _rewrite_chapter_links(render_markdown_to_html_python(md))
        if cid in {"satya_d20_rules", "tasks", "combat", "melee_combat"}:
            rendered = rendered + "\n" + _INTERACTIVE_WIDGET_HTML

        prev_spec = BOOK_CHAPTER_SPECS[idx - 1] if idx > 0 else None
        next_spec = BOOK_CHAPTER_SPECS[idx + 1] if idx + 1 < len(BOOK_CHAPTER_SPECS) else None
        page_html = _wrap_book_page_html(
            title=spec["title"],
            active_chapter_id=cid,
            body_html=rendered,
            prev_spec=prev_spec,
            next_spec=next_spec,
        )
        chapter_file = out_dir / f"{cid}.html"
        chapter_file.write_text(page_html, encoding="utf-8")
        generated_files.append(str(chapter_file))

        catalog_chapters.append(
            {
                "chapter_id": cid,
                "part": spec["part"],
                "title": spec["title"],
                "relative_path": spec["relative_path"],
                "description": spec["description"],
                "headings": headings,
                "html_file": f"{cid}.html",
            }
        )

    # Precompute a reference K=20 calibration table for the index page
    ref_rows: list[str] = []
    for c_val in (8, 10, 12, 14, 16):
        tn_eff = 31 - c_val
        for n_pairs in (2, 3, 4, 5):
            odds = paired_list_signed_margin_pmfs(
                num_pairs=n_pairs,
                sides=20,
                characteristic_c=tn_eff,
                delta_over=6,
                delta_under=6,
                mode="markov_step",
                count_under_as_success=False,
                botch_rule="closest_miss",
            )
            m_stats = compute_stats(odds.signed_margin_pmf)
            ref_rows.append(
                f"<tr><td><code>C={c_val}</code></td><td><code>TN_eff={tn_eff}</code></td>"
                f"<td><code>N={n_pairs}</code></td>"
                f"<td><strong>{odds.prob_any_success * 100:.1f}%</strong></td>"
                f"<td>{odds.prob_clean_failure * 100:.1f}%</td>"
                f"<td>{odds.prob_botch_failure * 100:.1f}%</td>"
                f"<td><code>{m_stats.mean:+.2f}</code></td></tr>"
            )

    toc_cards: list[str] = []
    for idx, ch in enumerate(catalog_chapters, start=1):
        toc_cards.append(
            f'<li><strong><a href="{ch["html_file"]}">{idx}. {html.escape(ch["title"])}</a></strong> '
            f'<span class="rules-inline-code">docs/ttrpg/{html.escape(ch["relative_path"])}</span><br/>'
            f'{html.escape(ch["description"])}</li>'
        )

    summary_md_path = root / "docs" / "ttrpg" / "SUMMARY.md"
    summary_html = (
        _rewrite_chapter_links(render_markdown_to_html_python(summary_md_path.read_text(encoding="utf-8")))
        if summary_md_path.exists()
        else ""
    )

    index_body = f"""
    <h1>Satya TTRPG Rules Book &amp; Interactive 2d20 Reference</h1>
    <p>
      Welcome to the static HTML &amp; Marimo Rules Book generated from <code>docs/ttrpg/</code>.
      Every chapter is also accessible inside the <strong>Satya Web Client (<code>Rules Book</code> Notebook Tab)</strong>,
      via the REST API (<code>GET /api/rules</code> and <code>GET /api/rules/:chapter_id</code>), and interactively
      in Marimo (<code>marimo run notebooks/satya_rules_book.py</code>).
    </p>
    {_INTERACTIVE_WIDGET_HTML}
    <h2>Chapter Directory</h2>
    <ul class="rules-list">{"".join(toc_cards)}</ul>
    <h2>Reference Calibration Table (<code>K = 20</code>, <code>TN_eff = 31 - C</code>, <code>&delta; = &pm;6</code>)</h2>
    <div class="rules-table-wrap">
      <table class="rules-table">
        <thead>
          <tr>
            <th>Control C</th><th>TN_eff</th><th>Skill N</th>
            <th>P(Success &ge; 1)</th><th>P(Clean Fail)</th><th>P(Botch Fail)</th><th>E[Margin]</th>
          </tr>
        </thead>
        <tbody>{"".join(ref_rows)}</tbody>
      </table>
    </div>
    {summary_html}
    """

    index_html = _wrap_book_page_html(
        title="Table of Contents & Interactive Sandbox",
        active_chapter_id="index",
        body_html=index_body,
        prev_spec=None,
        next_spec=BOOK_CHAPTER_SPECS[0],
    )
    index_path = out_dir / "index.html"
    index_path.write_text(index_html, encoding="utf-8")
    generated_files.append(str(index_path))

    catalog_payload = {
        "book_title": "Satya: A Grounded Tabletop Role-Playing System",
        "using_satya_py": using_satya_py_backend(),
        "chapter_count": len(catalog_chapters),
        "chapters": catalog_chapters,
    }
    catalog_path = out_dir / "catalog.json"
    catalog_path.write_text(json.dumps(catalog_payload, indent=2), encoding="utf-8")
    generated_files.append(str(catalog_path))

    return {
        "output_dir": str(out_dir),
        "chapter_count": len(catalog_chapters),
        "generated_files": generated_files,
        "using_satya_py": using_satya_py_backend(),
    }


if __name__ == "__main__":
    summary = build_static_rules_book()
    print(
        f"Built Satya Static Rules Book ({summary['chapter_count']} chapters) in {summary['output_dir']} "
        f"(satya_py active: {summary['using_satya_py']})"
    )
