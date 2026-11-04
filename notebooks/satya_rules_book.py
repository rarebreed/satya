import marimo

__generated_with = "0.21.1"
app = marimo.App(width="full")


@app.cell
def _():
    from pathlib import Path
    import re
    import sys

    import marimo as mo
    import plotly.graph_objects as go
    from plotly.subplots import make_subplots

    _mermaid_fence_re = re.compile(r"```mermaid[ \t]*\r?\n(.*?)```", re.DOTALL)

    def render_markdown_with_mermaid(md_text: str):
        """Render Markdown text while converting fenced ```mermaid blocks into native `mo.mermaid` diagrams."""
        parts = []
        last_idx = 0
        for match in _mermaid_fence_re.finditer(md_text):
            before = md_text[last_idx : match.start()].strip()
            if before:
                parts.append(mo.md(before))
            diagram_src = match.group(1).strip()
            if diagram_src:
                parts.append(mo.mermaid(diagram_src))
            last_idx = match.end()
        tail = md_text[last_idx:].strip()
        if tail:
            parts.append(mo.md(tail))
        if not parts:
            return mo.md(md_text)
        if len(parts) == 1:
            return parts[0]
        return mo.vstack(parts)

    repo_root = Path(__file__).resolve().parent.parent
    src_path = repo_root / "src"
    if str(src_path) not in sys.path:
        sys.path.insert(0, str(src_path))

    from satya_ml.dice_probabilities import (
        SATYA_PALETTE,
        compute_lifepath_skill_progression,
        compute_social_dissonance_check,
        compute_stats,
        compute_tactical_pool_envelope,
        evaluate_tactical_triad_contest,
        paired_list_signed_margin_pmfs,
        using_satya_py_backend,
    )

    ttrpg_dir = repo_root / "docs" / "ttrpg"
    CHAPTER_SPECS = [
        (
            "design_philosophy",
            "Part I: Foundation & Mathematical Calibration",
            "1. Design Philosophy & Goals of Satya",
            "01-foundation/design_philosophy.md",
        ),
        (
            "probability",
            "Part I: Foundation & Mathematical Calibration",
            "2. Probability & Mathematical Calibration",
            "01-foundation/probability_and_calibration.md",
        ),
        (
            "tasks",
            "Part II: Core Mechanics, Task Resolution & Action Economy",
            "3. Cascading 2d20 Engine & Task Resolution",
            "02-core-mechanics/cascading_d20_and_tasks.md",
        ),
        (
            "action_economy",
            "Part II: Core Mechanics, Task Resolution & Action Economy",
            "4. Action Economy, OODA Loop, Time Scales & Stealth",
            "02-core-mechanics/action_economy_and_ooda.md",
        ),
        (
            "characteristics",
            "Part III: Character Architecture, Skills, Lifepaths & Economy",
            "5. Characteristics, Senses, Psyche & Aptitudes",
            "03-character/characteristics_and_aptitudes.md",
        ),
        (
            "skills_and_trees",
            "Part III: Character Architecture, Skills, Lifepaths & Economy",
            "6. Skill System & Hierarchical Skill Trees",
            "03-character/skills_and_trees.md",
        ),
        (
            "lifepaths_and_economy",
            "Part III: Character Architecture, Skills, Lifepaths & Economy",
            "7. Lifepath Character Creation, Socioeconomic Upkeep & Heat",
            "03-character/lifepaths_and_economy.md",
        ),
        (
            "psyche_and_social",
            "Part IV: Psyche, Morale & Social Combat",
            "8. Psyche, Morale & Social Combat",
            "04-psyche-and-social/psyche_morale_and_social_combat.md",
        ),
        (
            "tactical_triad",
            "Part V: Combat Architecture, Close Quarters & Firefights",
            "9. Combat Architecture, Tactical Triad & Pools",
            "05-combat/tactical_triad_and_pools.md",
        ),
        (
            "melee_combat",
            "Part V: Combat Architecture, Close Quarters & Firefights",
            "10. Melee Combat, Close-Quarters Tactics & The Tactical Triad",
            "05-combat/melee_combat.md",
        ),
        (
            "ranged_combat_and_armor",
            "Part V: Combat Architecture, Close Quarters & Firefights",
            "11. Ranged Combat, Suppression, Armor & Weapon Reliability",
            "05-combat/ranged_combat_and_armor.md",
        ),
        (
            "health_and_trauma",
            "Part VI: Health, Trauma, Movement, Vehicles & Survival",
            "12. Health, Anatomical Trauma, Fatigue & Mental Conditions",
            "06-health-and-survival/health_and_trauma.md",
        ),
        (
            "movement_and_environment",
            "Part VI: Health, Trauma, Movement, Vehicles & Survival",
            "13. Movement, Encumbrance, Vehicles & Environmental Survival",
            "06-health-and-survival/movement_vehicles_and_environment.md",
        ),
    ]

    chapters_by_id = {}
    for _idx, (_cid, _part, _title, _rel_path) in enumerate(CHAPTER_SPECS, start=1):
        _md_path = ttrpg_dir / _rel_path
        _md_text = _md_path.read_text(encoding="utf-8") if _md_path.exists() else f"# {_title}\n\n*(Missing {_rel_path})*"
        chapters_by_id[_cid] = {
            "id": _cid,
            "order": _idx,
            "part": _part,
            "title": _title,
            "rel_path": f"docs/ttrpg/{_rel_path}",
            "markdown": _md_text,
            "word_count": len(_md_text.split()),
        }

    return (
        CHAPTER_SPECS,
        SATYA_PALETTE,
        chapters_by_id,
        compute_lifepath_skill_progression,
        compute_social_dissonance_check,
        compute_stats,
        compute_tactical_pool_envelope,
        evaluate_tactical_triad_contest,
        go,
        make_subplots,
        mo,
        paired_list_signed_margin_pmfs,
        render_markdown_with_mermaid,
        using_satya_py_backend,
    )


@app.cell
def _(CHAPTER_SPECS, chapters_by_id, mo, using_satya_py_backend):
    total_words = sum(ch["word_count"] for ch in chapters_by_id.values())
    backend_badge = (
        "Rust `satya_py` PyO3 Native Engine Active"
        if using_satya_py_backend()
        else "Pure-Python `satya_ml` Exact DP Engine Active"
    )
    chapter_options = {
        f"{spec[2]} ({spec[1]})": spec[0] for spec in CHAPTER_SPECS
    }
    chapter_options["All 9 Chapters (Complete Rules Compendium)"] = "ALL"

    chapter_dropdown = mo.ui.dropdown(
        options=chapter_options,
        value="3. Task Resolution & Skill Check Mechanics (Part I: Foundations & Core Resolution)",
        label="Select Rules Chapter",
    )
    search_input = mo.ui.text(
        value="",
        placeholder="Search across all 9 chapters (e.g., TN_eff, Tempo, Kuzushi, Dissonance)...",
        label="Search Rules Book",
    )

    mo.vstack(
        [
            mo.md(
                f"""
                # Satya TTRPG Rules Book & Interactive Mechanics Compendium
                **9 Canonical Chapters · {total_words:,} Words · Engine Backend: `{backend_badge}`**

                Browse individual chapters from `docs/ttrpg/` below, search across the entire ruleset, or experiment directly with the four interactive mechanics laboratories (**2d20 Watershed Cascade**, **Lifepath Skill Progression**, **Combat Tactical Pool & Melee Triad**, and **Social Cognitive Dissonance**).
                """
            ),
            mo.hstack([chapter_dropdown, search_input], justify="start"),
        ]
    )
    return chapter_dropdown, search_input


@app.cell
def _(
    chapter_dropdown,
    chapters_by_id,
    mo,
    render_markdown_with_mermaid,
    search_input,
):
    selected_id = chapter_dropdown.value
    query_str = (search_input.value or "").strip().lower()

    blocks = []
    if query_str:
        hits = []
        for _cid, _ch in chapters_by_id.items():
            for _sec in _ch["markdown"].split("\n## "):
                _chunk = _sec.strip()
                if query_str in _chunk.lower():
                    _first_line = _chunk.splitlines()[0].lstrip("# ").strip()
                    _snippet = " ".join(_chunk.split())[:260]
                    hits.append(f"- **{_ch['title']}** (`{_ch['rel_path']}`) — *{_first_line}*: {_snippet}...")
        if hits:
            blocks.append(
                mo.md(
                    f"### Search Results for `{query_str}` ({len(hits)} section matches)\n"
                    + "\n".join(hits[:10])
                )
            )
        else:
            blocks.append(mo.md(f"*No sections matched `{query_str}`.*"))

    if selected_id == "ALL":
        for _cid, _ch in chapters_by_id.items():
            blocks.append(
                render_markdown_with_mermaid(
                    f"---\n*Part: {_ch['part']} · Source: `{_ch['rel_path']}` ({_ch['word_count']:,} words)*\n\n"
                    + _ch["markdown"]
                )
            )
    else:
        _ch = chapters_by_id.get(selected_id, chapters_by_id["tasks"])
        blocks.append(
            render_markdown_with_mermaid(
                f"*Part: {_ch['part']} · Source: `{_ch['rel_path']}` ({_ch['word_count']:,} words)*\n\n"
                + _ch["markdown"]
            )
        )

    mo.vstack(blocks)
    return


@app.cell
def _(mo):
    preset_dropdown = mo.ui.dropdown(
        options={
            "Custom Slider Parameters": "custom",
            "Ch. 3 Example: Sarah's Rusted Lockpick (N=4, C=14, Δ_int=+1, Δ_ext=+4)": "sarah",
            "Ch. 8 Example: Elena's Arming Sword Probe (N=5, C=16, Δ_int=0, Δ_ext=+2)": "elena",
            "Ch. 8 Example: Viktor's Greatsword Cleave (N=4, C=13, Δ_int=0, Δ_ext=+3)": "viktor",
            "Ch. 9 Example: Counter-Interrogation Rhetoric (N=3, C=15, Δ_int=+2, Δ_ext=+3)": "social",
        },
        value="Ch. 3 Example: Sarah's Rusted Lockpick (N=4, C=14, Δ_int=+1, Δ_ext=+4)",
        label="Book Worked Example Preset",
    )
    skill_n_slider = mo.ui.slider(start=1, stop=10, step=1, value=4, label="Skill Level N (2d20 Pairs)")
    control_c_slider = mo.ui.slider(start=1, stop=20, step=1, value=14, label="Controlling Characteristic C (1..20)")
    delta_int_slider = mo.ui.slider(start=-2, stop=8, step=1, value=1, label="Internal Mod Δ_int (Fatigue / Shock / Tool Penalty)")
    delta_ext_slider = mo.ui.slider(start=-6, stop=12, step=1, value=4, label="External Mod Δ_ext (Task Difficulty)")
    carry_mode_dropdown = mo.ui.dropdown(
        options=["markov_step", "cumulative_snowball", "linear_margin"],
        value="markov_step",
        label="Cascade Carry Mode",
    )
    botch_rule_dropdown = mo.ui.dropdown(
        options=["closest_miss", "ceiling_breach", "overrun_k"],
        value="closest_miss",
        label="Botch / Failure Severity Rule",
    )

    mo.vstack(
        [
            mo.md(
                r"""
                ---
                ## Interactive Lab 1: `2d20` Two-List Cascading ("Watershed") Task Simulator (Chapters 2–4)
                $$\text{TN}_{\text{eff}} = 31 - C + \Delta_{\text{ext}}, \qquad \delta_{\text{over}} = 6, \qquad \delta_{\text{under}} = \max(0, 6 + \Delta_{\text{int}})$$
                """
            ),
            preset_dropdown,
            mo.hstack(
                [
                    mo.vstack([skill_n_slider, control_c_slider, delta_int_slider]),
                    mo.vstack([delta_ext_slider, carry_mode_dropdown, botch_rule_dropdown]),
                ],
                justify="space-around",
            ),
        ]
    )
    return (
        botch_rule_dropdown,
        carry_mode_dropdown,
        control_c_slider,
        delta_ext_slider,
        delta_int_slider,
        preset_dropdown,
        skill_n_slider,
    )


@app.cell
def _(
    SATYA_PALETTE,
    botch_rule_dropdown,
    carry_mode_dropdown,
    compute_stats,
    control_c_slider,
    delta_ext_slider,
    delta_int_slider,
    go,
    make_subplots,
    mo,
    paired_list_signed_margin_pmfs,
    preset_dropdown,
    skill_n_slider,
):
    preset_map = {
        "sarah": (4, 14, 1, 4),
        "elena": (5, 16, 0, 2),
        "viktor": (4, 13, 0, 3),
        "social": (3, 15, 2, 3),
    }
    if preset_dropdown.value in preset_map:
        n_pairs, c_val, d_int, d_ext = preset_map[preset_dropdown.value]
    else:
        n_pairs = int(skill_n_slider.value)
        c_val = int(control_c_slider.value)
        d_int = int(delta_int_slider.value)
        d_ext = int(delta_ext_slider.value)

    tn_eff = 31 - c_val + d_ext
    d_over = 6
    d_under = max(0, 6 + d_int)
    res = paired_list_signed_margin_pmfs(
        num_pairs=n_pairs,
        sides=20,
        characteristic_c=tn_eff,
        delta_over=d_over,
        delta_under=d_under,
        mode=carry_mode_dropdown.value,
        count_under_as_success=False,
        botch_rule=botch_rule_dropdown.value,
    )
    m_stats = compute_stats(res.signed_margin_pmf)

    fig = make_subplots(
        rows=1,
        cols=2,
        subplot_titles=(
            f"Signed Margin Spectrum (TN_eff={tn_eff}, P(Succ≥1)={res.prob_any_success*100:.1f}%)",
            f"Final Pair Effective Value Y_{n_pairs} vs. TN_eff={tn_eff}",
        ),
    )
    m_xs = list(res.signed_margin_pmf.keys())
    m_ys = [res.signed_margin_pmf[x] * 100.0 for x in m_xs]
    m_colors = [
        "#f43f5e" if x < 0 else ("#94a3b8" if x == 0 else "#34d399")
        for x in m_xs
    ]
    fig.add_trace(
        go.Bar(x=m_xs, y=m_ys, marker_color=m_colors, name="Signed Margin"),
        row=1,
        col=1,
    )
    y_xs = list(res.final_pair_value_pmf.keys())
    y_ys = [res.final_pair_value_pmf[x] * 100.0 for x in y_xs]
    fig.add_trace(
        go.Scatter(
            x=y_xs,
            y=y_ys,
            mode="lines+markers",
            line=dict(color=SATYA_PALETTE[0], width=2.5),
            name=f"Final Pair Y_{n_pairs}",
        ),
        row=1,
        col=2,
    )
    fig.add_vline(x=tn_eff, line_dash="dash", line_color="#fbbf24", row=1, col=2)
    fig.update_layout(
        template="plotly_dark",
        paper_bgcolor="#0f172a",
        plot_bgcolor="#1e293b",
        height=400,
    )

    mo.vstack(
        [
            mo.md(
                f"""
                **Active Parameters**: `N = {n_pairs}` (`2d20` pairs) · `C = {c_val}` · `Δ_int = {d_int:+d}` (`δ_under = {d_under}`) · `Δ_ext = {d_ext:+d}` $\\implies$ **`TN_eff = {tn_eff}`**
                - **Any Success (`≥ +1`)**: `{res.prob_any_success * 100:.2f}%` | **Success w/ Complication**: `{res.prob_success_with_complication * 100:.2f}%`
                - **Clean Miss (`0`)**: `{res.prob_clean_failure * 100:.2f}%` | **Botch Failure (`< 0`)**: `{res.prob_botch_failure * 100:.2f}%` | **Expected Margin**: `{m_stats.mean:+.2f}` (`std = {m_stats.std:.2f}`)
                """
            ),
            mo.ui.plotly(fig),
        ]
    )
    return


@app.cell
def _(
    compute_lifepath_skill_progression,
    compute_social_dissonance_check,
    compute_tactical_pool_envelope,
    evaluate_tactical_triad_contest,
    mo,
):
    # Lab 2: Lifepath Progression Controls
    study_hours_slider = mo.ui.slider(
        start=50, stop=12000, step=50, value=1600, label="Cumulative Practice / Study Hours"
    )
    instruction_slider = mo.ui.slider(
        start=0.5, stop=2.0, step=0.1, value=1.2, label="Mentor / Instruction Quality Multiplier"
    )
    complexity_slider = mo.ui.slider(
        start=0.5, stop=2.0, step=0.1, value=1.0, label="Skill Complexity Multiplier"
    )

    # Lab 3: Tactical Pool & Melee Triad Controls
    combat_c_slider = mo.ui.slider(start=8, stop=20, step=1, value=16, label="Attacker Control C")
    combat_n_slider = mo.ui.slider(start=1, stop=10, step=1, value=5, label="Attacker Melee Skill N")
    combat_succ_slider = mo.ui.slider(start=0, stop=6, step=1, value=3, label="Cascade Successes Rolled")
    atk_tempo_slider = mo.ui.slider(start=0, stop=10, step=1, value=3, label="Attacker Tempo (T)")
    atk_ctrl_slider = mo.ui.slider(start=0, stop=10, step=1, value=4, label="Attacker Control (C)")
    atk_force_slider = mo.ui.slider(start=0, stop=10, step=1, value=3, label="Attacker Force (F)")
    def_tempo_slider = mo.ui.slider(start=0, stop=10, step=1, value=2, label="Defender Tempo (T)")
    def_ctrl_slider = mo.ui.slider(start=0, stop=10, step=1, value=3, label="Defender Control (C)")
    def_force_slider = mo.ui.slider(start=0, stop=10, step=1, value=2, label="Defender Force (F)")

    # Lab 4: Social Dissonance Controls
    soc_margin_slider = mo.ui.slider(start=-3, stop=6, step=1, value=3, label="Rhetoric Cascade Margin")
    soc_conviction_slider = mo.ui.slider(start=1, stop=20, step=1, value=12, label="Target Core Conviction (1..20)")
    soc_evidence_slider = mo.ui.slider(start=0, stop=5, step=1, value=2, label="Verifiable Evidence Tier (0..5)")

    mo.vstack(
        [
            mo.md("---\n## Interactive Labs 2, 3 & 4: Lifepath Study, Tactical Combat Triad & Social Dissonance"),
            mo.hstack(
                [
                    mo.vstack(
                        [
                            mo.md("**Lab 2: Lifepath Skill Study (Ch. 6)**"),
                            study_hours_slider,
                            instruction_slider,
                            complexity_slider,
                        ]
                    ),
                    mo.vstack(
                        [
                            mo.md("**Lab 3: Combat Pool & Triad (Ch. 7–8)**"),
                            combat_c_slider,
                            combat_n_slider,
                            combat_succ_slider,
                            atk_tempo_slider,
                            atk_ctrl_slider,
                            atk_force_slider,
                            def_tempo_slider,
                            def_ctrl_slider,
                            def_force_slider,
                        ]
                    ),
                    mo.vstack(
                        [
                            mo.md("**Lab 4: Social Dissonance (Ch. 9)**"),
                            soc_margin_slider,
                            soc_conviction_slider,
                            soc_evidence_slider,
                        ]
                    ),
                ],
                justify="space-around",
            ),
        ]
    )
    return (
        atk_ctrl_slider,
        atk_force_slider,
        atk_tempo_slider,
        combat_c_slider,
        combat_n_slider,
        combat_succ_slider,
        complexity_slider,
        def_ctrl_slider,
        def_force_slider,
        def_tempo_slider,
        instruction_slider,
        soc_conviction_slider,
        soc_evidence_slider,
        soc_margin_slider,
        study_hours_slider,
    )


@app.cell
def _(
    atk_ctrl_slider,
    atk_force_slider,
    atk_tempo_slider,
    combat_c_slider,
    combat_n_slider,
    combat_succ_slider,
    complexity_slider,
    compute_lifepath_skill_progression,
    compute_social_dissonance_check,
    compute_tactical_pool_envelope,
    def_ctrl_slider,
    def_force_slider,
    def_tempo_slider,
    evaluate_tactical_triad_contest,
    instruction_slider,
    mo,
    soc_conviction_slider,
    soc_evidence_slider,
    soc_margin_slider,
    study_hours_slider,
):
    lp = compute_lifepath_skill_progression(
        hours_studied=study_hours_slider.value,
        difficulty_multiplier=complexity_slider.value,
        instruction_quality=instruction_slider.value,
    )
    pool = compute_tactical_pool_envelope(
        control_c=combat_c_slider.value,
        skill_level=combat_n_slider.value,
        num_successes=combat_succ_slider.value,
    )
    triad = evaluate_tactical_triad_contest(
        attacker_tempo=atk_tempo_slider.value,
        attacker_control=atk_ctrl_slider.value,
        attacker_force=atk_force_slider.value,
        defender_tempo=def_tempo_slider.value,
        defender_control=def_ctrl_slider.value,
        defender_force=def_force_slider.value,
    )
    soc = compute_social_dissonance_check(
        argument_margin=soc_margin_slider.value,
        target_conviction=soc_conviction_slider.value,
        evidence_tier=soc_evidence_slider.value,
    )

    mo.md(
        f"""
        | Mechanics Laboratory | Computed Telemetry | Canonical Resolution Outcome |
        | :--- | :--- | :--- |
        | **Lab 2: Lifepath Progression (Ch. 6)** | Raw `{lp['raw_hours']}h` $\\to$ Effective `{lp['effective_hours']}h` | **Skill Rank `N = {lp['skill_level_n']}`** (`{lp['hours_to_next_level']}h` to Rank `N = {lp['next_level_n']}`) |
        | **Lab 3A: Tactical Pool Envelope (Ch. 7)** | Base Envelope `E_base = {pool['base_envelope']}` · Raw Fill `{pool['raw_pool']}` | **Effective Pool `P_eff = {pool['effective_pool']}`** (Overflow wasted: `{pool['overflow_wasted']}`) |
        | **Lab 3B: Close-Quarters Triad (Ch. 8)** | Net Tempo `{triad['net_tempo']:+d}` · Net Control `{triad['net_control']:+d}` · Penetrating Force `{triad['penetrating_force']}` | **{triad['outcome_summary']}** (Next Action Cost: `{triad['next_tick_cost']} ticks`) |
        | **Lab 4: Social Dissonance (Ch. 9)** | Dissonance Pressure `{soc['dissonance_pressure']}` vs. Threshold `{soc['resistance_threshold']}` (Net `{soc['net_dissonance_shift']:+d}`) | **{soc['social_outcome']}** |
        """
    )
    return


if __name__ == "__main__":
    app.run()
