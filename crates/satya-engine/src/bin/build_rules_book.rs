//! CLI utility (`cargo run -p satya-engine --bin build-rules-book`) that compiles all 9 canonical
//! chapters of the Satya TTRPG Rules Book (`docs/ttrpg/`) into static HTML pages and `catalog.json`
//! inside `docs/book/`.

#![deny(missing_docs)]

use satya_engine::{
    BotchRule, CarryMode, MarginAggregation, canonical_rules_chapters, compute_stats,
    escape_html_text, paired_list_signed_margin_pmfs, render_rules_markdown_to_html,
};
use serde_json::json;
use std::fs;
use std::path::Path;

const INTERACTIVE_WIDGET_HTML: &str = r##"
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
        <button type="button" onclick="satyaBookLoadPreset(4,11,2,2,'14+11, 8+7, 16+15, 17+12')">Sarah's Lockpick</button>
        <button type="button" onclick="satyaBookLoadPreset(5,16,0,2,'12+10, 9+11, 14+8, 16+9, 11+12')">Elena's Probe</button>
        <button type="button" onclick="satyaBookLoadPreset(4,13,0,3,'15+11, 7+6, 13+10, 18+9')">Viktor's Cleave</button>
        <button type="button" onclick="satyaBookRandomRoll()">Roll Random N&times;2d20</button>
      </div>
      <label class="full-width">Rolled 2d20 Pairs (A+B, comma-separated):
        <input id="bk-pairs" type="text" value="14+11, 8+7, 16+15, 17+12" oninput="satyaBookRecalc()"/>
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
"##;

fn rewrite_chapter_links(rendered_html: &str) -> String {
    let mapping = [
        (
            "01-foundation/design_philosophy.md",
            "design_philosophy.html",
        ),
        (
            "../01-foundation/design_philosophy.md",
            "design_philosophy.html",
        ),
        ("design_philosophy.md", "design_philosophy.html"),
        (
            "01-foundation/mathematical_notation_glossary.md",
            "notation_glossary.html",
        ),
        (
            "../01-foundation/mathematical_notation_glossary.md",
            "notation_glossary.html",
        ),
        (
            "mathematical_notation_glossary.md",
            "notation_glossary.html",
        ),
        (
            "01-foundation/probability_and_calibration.md",
            "probability.html",
        ),
        (
            "../01-foundation/probability_and_calibration.md",
            "probability.html",
        ),
        ("probability_and_calibration.md", "probability.html"),
        ("02-core-mechanics/cascading_d20_and_tasks.md", "tasks.html"),
        (
            "../02-core-mechanics/cascading_d20_and_tasks.md",
            "tasks.html",
        ),
        ("cascading_d20_and_tasks.md", "tasks.html"),
        (
            "02-core-mechanics/task_requirements_catalog.md",
            "task_requirements_catalog.html",
        ),
        (
            "../02-core-mechanics/task_requirements_catalog.md",
            "task_requirements_catalog.html",
        ),
        (
            "task_requirements_catalog.md",
            "task_requirements_catalog.html",
        ),
        (
            "02-core-mechanics/action_economy_and_ooda.md",
            "action_economy.html",
        ),
        (
            "../02-core-mechanics/action_economy_and_ooda.md",
            "action_economy.html",
        ),
        ("action_economy_and_ooda.md", "action_economy.html"),
        (
            "03-character/characteristics_and_aptitudes.md",
            "characteristics.html",
        ),
        (
            "../03-character/characteristics_and_aptitudes.md",
            "characteristics.html",
        ),
        ("characteristics_and_aptitudes.md", "characteristics.html"),
        ("03-character/skills_and_trees.md", "skills_and_trees.html"),
        (
            "../03-character/skills_and_trees.md",
            "skills_and_trees.html",
        ),
        ("skills_and_trees.md", "skills_and_trees.html"),
        (
            "03-character/skills_trees_tactical_and_social.md",
            "skills_trees_tactical_and_social.html",
        ),
        (
            "../03-character/skills_trees_tactical_and_social.md",
            "skills_trees_tactical_and_social.html",
        ),
        (
            "skills_trees_tactical_and_social.md",
            "skills_trees_tactical_and_social.html",
        ),
        (
            "03-character/lifepaths_and_economy.md",
            "lifepaths_and_economy.html",
        ),
        (
            "../03-character/lifepaths_and_economy.md",
            "lifepaths_and_economy.html",
        ),
        ("lifepaths_and_economy.md", "lifepaths_and_economy.html"),
        (
            "04-psyche-and-social/psyche_morale_and_social_combat.md",
            "psyche_and_social.html",
        ),
        (
            "../04-psyche-and-social/psyche_morale_and_social_combat.md",
            "psyche_and_social.html",
        ),
        (
            "psyche_morale_and_social_combat.md",
            "psyche_and_social.html",
        ),
        (
            "05-combat/tactical_triad_and_pools.md",
            "tactical_triad.html",
        ),
        (
            "../05-combat/tactical_triad_and_pools.md",
            "tactical_triad.html",
        ),
        ("tactical_triad_and_pools.md", "tactical_triad.html"),
        ("05-combat/melee_combat.md", "melee_combat.html"),
        ("../05-combat/melee_combat.md", "melee_combat.html"),
        ("melee_combat.md", "melee_combat.html"),
        (
            "05-combat/firearms_and_ballistics.md",
            "firearms_and_ballistics.html",
        ),
        (
            "../05-combat/firearms_and_ballistics.md",
            "firearms_and_ballistics.html",
        ),
        (
            "firearms_and_ballistics.md",
            "firearms_and_ballistics.html",
        ),
        (
            "05-combat/ranged_combat_and_armor.md",
            "ranged_combat_and_armor.html",
        ),
        (
            "../05-combat/ranged_combat_and_armor.md",
            "ranged_combat_and_armor.html",
        ),
        ("ranged_combat_and_armor.md", "ranged_combat_and_armor.html"),
        (
            "06-health-and-survival/health_and_trauma.md",
            "health_and_trauma.html",
        ),
        (
            "../06-health-and-survival/health_and_trauma.md",
            "health_and_trauma.html",
        ),
        ("health_and_trauma.md", "health_and_trauma.html"),
        (
            "06-health-and-survival/movement_vehicles_and_environment.md",
            "movement_and_environment.html",
        ),
        (
            "../06-health-and-survival/movement_vehicles_and_environment.md",
            "movement_and_environment.html",
        ),
        (
            "movement_vehicles_and_environment.md",
            "movement_and_environment.html",
        ),
    ];
    let mut out = rendered_html.to_string();
    for (src, dst) in mapping {
        out = out.replace(&format!("href=\"{src}\""), &format!("href=\"{dst}\""));
        out = out.replace(&format!("href=\"{src}#"), &format!("href=\"{dst}#"));
    }
    out
}

fn wrap_book_page_html(
    title: &str,
    active_chapter_id: &str,
    body_html: &str,
    chapters: &[satya_schema::RulesChapterDocument],
    prev_idx: Option<usize>,
    next_idx: Option<usize>,
) -> String {
    let mut nav_items = String::new();
    let idx_cls = if active_chapter_id == "index" {
        "book-nav-link active"
    } else {
        "book-nav-link"
    };
    nav_items.push_str(&format!(
        "<a href=\"index.html\" class=\"{idx_cls}\">0. Table of Contents &amp; Interactive Sandbox</a>"
    ));

    let mut current_part = String::new();
    for (i, doc) in chapters.iter().enumerate() {
        if doc.summary.part != current_part {
            current_part = doc.summary.part.clone();
            nav_items.push_str(&format!(
                "<div class=\"book-nav-part\">{}</div>",
                escape_html_text(&current_part)
            ));
        }
        let cls = if doc.summary.chapter_id == active_chapter_id {
            "book-nav-link active"
        } else {
            "book-nav-link"
        };
        nav_items.push_str(&format!(
            "<a href=\"{}.html\" class=\"{cls}\">{}. {}</a>",
            doc.summary.chapter_id,
            i + 1,
            escape_html_text(&doc.summary.title)
        ));
    }

    let mut footer_links = String::new();
    if let Some(pi) = prev_idx {
        let p = &chapters[pi].summary;
        footer_links.push_str(&format!(
            "<a class=\"book-pager-btn\" href=\"{}.html\">&larr; Previous: {}</a>",
            p.chapter_id,
            escape_html_text(&p.title)
        ));
    } else {
        footer_links.push_str(
            "<a class=\"book-pager-btn\" href=\"index.html\">&larr; Table of Contents</a>",
        );
    }
    if let Some(ni) = next_idx {
        let n = &chapters[ni].summary;
        footer_links.push_str(&format!(
            "<a class=\"book-pager-btn\" href=\"{}.html\">Next: {} &rarr;</a>",
            n.chapter_id,
            escape_html_text(&n.title)
        ));
    }

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8"/>
  <meta name="viewport" content="width=device-width, initial-scale=1"/>
  <title>{} — Satya TTRPG Rules Book</title>
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
    .book-brand {{ font-size: 1.15rem; font-weight: 700; color: var(--accent); margin-bottom: 4px; }}
    .book-sub {{ font-size: 0.8rem; color: var(--muted); margin-bottom: 16px; }}
    .book-nav-part {{ font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.06em; color: var(--muted); margin: 14px 0 6px; font-weight: 600; }}
    .book-nav-link {{ display: block; padding: 7px 10px; border-radius: 6px; color: var(--text); text-decoration: none; font-size: 0.88rem; margin-bottom: 3px; }}
    .book-nav-link:hover {{ background: rgba(56, 189, 248, 0.12); }}
    .book-nav-link.active {{ background: rgba(56, 189, 248, 0.2); color: var(--accent); font-weight: 600; }}
    main.book-main {{ max-width: 980px; padding: 32px 44px 64px; line-height: 1.65; }}
    h1, h2, h3, h4 {{ color: #f8fafc; }}
    h1 {{ border-bottom: 1px solid var(--border); padding-bottom: 10px; }}
    h2 {{ margin-top: 2rem; border-bottom: 1px solid var(--border); padding-bottom: 6px; }}
    a {{ color: var(--accent); }}
    code, pre {{ font-family: "JetBrains Mono", Consolas, monospace; }}
    .rules-inline-code {{ background: #1e293b; padding: 2px 6px; border-radius: 4px; font-size: 0.88em; }}
    .rules-code-block {{ background: #0f172a; border: 1px solid var(--border); padding: 14px; border-radius: 8px; overflow-x: auto; }}
    .rules-table-wrap {{ overflow-x: auto; margin: 14px 0; }}
    .rules-table {{ width: 100%; border-collapse: collapse; font-size: 0.9rem; }}
    .rules-table th, .rules-table td {{ border: 1px solid var(--border); padding: 8px 10px; text-align: left; }}
    .rules-table th {{ background: #1e293b; color: var(--accent); }}
    .rules-alert, .rules-blockquote {{ border-left: 4px solid var(--accent); background: rgba(30, 41, 59, 0.55); padding: 10px 16px; border-radius: 0 8px 8px 0; margin: 14px 0; }}
    .rules-alert-title {{ font-weight: 700; color: var(--accent); margin-bottom: 4px; }}
    .rules-math-inline {{ font-family: "Cambria Math", "STIX Two Math", Georgia, serif; font-size: 0.98em; color: #f8fafc; background: rgba(56, 189, 248, 0.08); padding: 0 4px; border-radius: 3px; }}
    .rules-math-block {{ background: #111827; border: 1px solid var(--border); border-left: 3px solid var(--accent); border-radius: 6px; padding: 10px 14px; margin: 10px 0; font-family: "Cambria Math", "STIX Two Math", Georgia, serif; font-size: 0.96rem; color: #f8fafc; text-align: center; overflow-x: auto; }}
    .math-roman, .math-fn {{ font-family: Inter, system-ui, sans-serif; font-style: normal; }}
    .math-fn {{ font-weight: 600; color: #7dd3fc; }}
    .math-frac {{ display: inline-flex; flex-direction: column; align-items: center; vertical-align: middle; margin: 0 3px; line-height: 1.1; font-size: 0.88em; }}
    .math-num {{ border-bottom: 1px solid currentColor; padding: 0 3px 1px; }}
    .math-den {{ padding: 1px 3px 0; }}
    .math-cases {{ display: inline-flex; align-items: center; vertical-align: middle; gap: 6px; }}
    .math-brace {{ font-size: 1.65em; line-height: 1; }}
    .math-cases-rows {{ display: inline-flex; flex-direction: column; gap: 2px; text-align: left; }}
    .rules-mermaid {{ background: #0b0f19; border: 1px solid var(--border); border-radius: 8px; padding: 14px; margin: 14px 0; max-width: 100%; overflow-x: auto; display: flex; flex-direction: column; gap: 10px; }}
    .mermaid-top-nodes {{ display: flex; flex-wrap: wrap; gap: 8px; justify-content: center; }}
    .mermaid-flow-track {{ display: flex; flex-wrap: wrap; align-items: stretch; gap: 8px; }}
    .mermaid-dir-td > .mermaid-flow-track {{ flex-direction: column; }}
    .mermaid-subgraphs-grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(min(100%, 240px), 1fr)); gap: 12px; align-items: start; }}
    .mermaid-subgraph {{ background: #111827; border: 1px solid var(--border); border-top: 2px solid var(--accent); border-radius: 7px; padding: 10px; display: flex; flex-direction: column; gap: 8px; }}
    .mermaid-subgraph-title {{ font-size: 0.76rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.04em; color: var(--accent); padding-bottom: 4px; border-bottom: 1px dashed var(--border); }}
    .mermaid-subgraph-nodes {{ display: flex; flex-direction: column; gap: 8px; }}
    .mermaid-node {{ flex: 1 1 180px; background: #1e293b; border: 1px solid rgba(56, 189, 248, 0.35); border-radius: 6px; padding: 9px 11px; font-size: 0.82rem; line-height: 1.42; color: var(--text); display: flex; flex-direction: column; justify-content: space-between; gap: 6px; }}
    .mermaid-node-diamond {{ border-color: #f59e0b; background: rgba(245, 158, 11, 0.1); }}
    .mermaid-connector {{ display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 2px; padding: 2px 4px; color: var(--accent); font-weight: 700; }}
    .mermaid-node-links {{ display: flex; flex-wrap: wrap; gap: 4px; padding-top: 5px; border-top: 1px dashed rgba(148, 163, 184, 0.25); }}
    .mermaid-link-pill {{ display: inline-flex; align-items: center; gap: 4px; background: #090d16; border: 1px solid var(--border); border-radius: 4px; padding: 2px 6px; font-size: 0.72rem; color: #94a3b8; }}
    .mermaid-edge-label {{ color: var(--emerald); font-weight: 600; font-size: 0.72rem; }}
    .book-interactive-widget {{ background: #111827; border: 1px solid #334155; border-radius: 10px; padding: 20px; margin: 24px 0; }}
    .widget-grid {{ display: grid; display: grid; grid-template-columns: 1fr 1fr; gap: 18px; }}
    .widget-panel {{ background: #0f172a; border: 1px solid var(--border); border-radius: 8px; padding: 14px; }}
    .widget-controls {{ display: grid; grid-template-columns: 1fr 1fr; gap: 8px; margin-bottom: 10px; }}
    .widget-controls label, label.full-width {{ display: flex; flex-direction: column; font-size: 0.8rem; color: var(--muted); gap: 4px; }}
    .widget-controls input, label.full-width input {{ background: #1e293b; border: 1px solid #334155; color: var(--text); padding: 6px 8px; border-radius: 4px; }}
    .widget-presets {{ display: flex; flex-wrap: wrap; gap: 6px; margin: 8px 0; }}
    .widget-presets button {{ background: #1e293b; border: 1px solid #38bdf8; color: var(--accent); padding: 5px 10px; border-radius: 5px; cursor: pointer; font-size: 0.8rem; }}
    .book-pager {{ display: flex; justify-content: space-between; margin-top: 40px; padding-top: 16px; border-top: 1px solid var(--border); }}
    .book-pager-btn {{ background: var(--panel); border: 1px solid var(--border); padding: 8px 14px; border-radius: 6px; text-decoration: none; }}
  </style>
</head>
<body>
  <aside class="book-sidebar">
    <div class="book-brand">SATYA // RULES BOOK</div>
    <div class="book-sub">2d20 Watershed Cascade &amp; Tactical Triad</div>
    {nav_items}
  </aside>
  <main class="book-main">
    {body_html}
    <div class="book-pager">
      {footer_links}
    </div>
  </main>
</body>
</html>"#,
        escape_html_text(title)
    )
}

fn main() {
    let out_dir = Path::new("docs").join("book");
    fs::create_dir_all(&out_dir).expect("failed to create docs/book directory");

    let chapters = canonical_rules_chapters();
    let mut catalog_entries = Vec::new();

    for (idx, doc) in chapters.iter().enumerate() {
        let cid = &doc.summary.chapter_id;
        let mut body = rewrite_chapter_links(&doc.rendered_html);
        if matches!(
            cid.as_str(),
            "tasks" | "tactical_triad" | "melee_combat" | "ranged_combat_and_armor"
        ) {
            body.push('\n');
            body.push_str(INTERACTIVE_WIDGET_HTML);
        }
        let prev_idx = idx.checked_sub(1);
        let next_idx = if idx + 1 < chapters.len() {
            Some(idx + 1)
        } else {
            None
        };
        let page = wrap_book_page_html(
            &doc.summary.title,
            cid,
            &body,
            &chapters,
            prev_idx,
            next_idx,
        );
        fs::write(out_dir.join(format!("{cid}.html")), page).expect("failed to write chapter html");

        catalog_entries.push(json!({
            "chapter_id": doc.summary.chapter_id,
            "part": doc.summary.part,
            "title": doc.summary.title,
            "relative_path": doc.summary.relative_path,
            "description": doc.summary.description,
            "headings": doc.summary.headings,
            "html_file": format!("{cid}.html"),
        }));
    }

    let mut ref_rows = String::new();
    for c_val in [8, 10, 12, 14, 16] {
        let tn_eff = 31 - c_val;
        for n_pairs in [2usize, 3, 4, 5] {
            let odds = paired_list_signed_margin_pmfs(
                n_pairs,
                20,
                tn_eff,
                6,
                6,
                CarryMode::MarkovStep,
                false,
                false,
                BotchRule::ClosestMiss,
                MarginAggregation::PiecewiseZeroFloor,
            );
            let stats = compute_stats(&odds.signed_margin_pmf);
            ref_rows.push_str(&format!(
                "<tr><td><code>C={c_val}</code></td><td><code>TN_eff={tn_eff}</code></td><td><code>N={n_pairs}</code></td><td><strong>{:.1}%</strong></td><td>{:.1}%</td><td>{:.1}%</td><td><code>{:+.2}</code></td></tr>",
                odds.prob_any_success * 100.0,
                odds.prob_clean_failure * 100.0,
                odds.prob_botch_failure * 100.0,
                stats.mean
            ));
        }
    }

    let mut toc_cards = String::new();
    for (idx, doc) in chapters.iter().enumerate() {
        toc_cards.push_str(&format!(
            "<li><strong><a href=\"{}.html\">{}. {}</a></strong> <span class=\"rules-inline-code\">docs/ttrpg/{}</span><br/>{}</li>",
            doc.summary.chapter_id,
            idx + 1,
            escape_html_text(&doc.summary.title),
            escape_html_text(&doc.summary.relative_path),
            escape_html_text(&doc.summary.description)
        ));
    }

    let summary_md =
        fs::read_to_string(Path::new("docs").join("ttrpg").join("SUMMARY.md")).unwrap_or_default();
    let summary_html = rewrite_chapter_links(&render_rules_markdown_to_html(&summary_md));

    let index_body = format!(
        r#"<h1>Satya TTRPG Rules Book &amp; Interactive 2d20 Reference</h1>
<p>
  Welcome to the static HTML &amp; Marimo Rules Book generated from <code>docs/ttrpg/</code>.
  Every chapter is also accessible inside the <strong>Satya Web Client (<code>Rules Book</code> Notebook Tab)</strong>,
  via the REST API (<code>GET /api/rules</code> and <code>GET /api/rules/:chapter_id</code>), and interactively
  in Marimo (<code>marimo run notebooks/satya_rules_book.py</code>).
</p>
{INTERACTIVE_WIDGET_HTML}
<h2>Chapter Directory</h2>
<ul class="rules-list">{toc_cards}</ul>
<h2>Reference Calibration Table (<code>K = 20</code>, <code>TN_eff = 31 - C</code>, <code>&delta; = &pm;6</code>)</h2>
<div class="rules-table-wrap">
  <table class="rules-table">
    <thead>
      <tr>
        <th>Control C</th><th>TN_eff</th><th>Skill N</th>
        <th>P(Success &ge; 1)</th><th>P(Clean Fail)</th><th>P(Botch Fail)</th><th>E[Margin]</th>
      </tr>
    </thead>
    <tbody>{ref_rows}</tbody>
  </table>
</div>
{summary_html}"#
    );

    let index_html = wrap_book_page_html(
        "Table of Contents & Interactive Sandbox",
        "index",
        &index_body,
        &chapters,
        None,
        Some(0),
    );
    fs::write(out_dir.join("index.html"), index_html)
        .expect("failed to write docs/book/index.html");

    let catalog_json = json!({
        "book_title": "Satya: A Grounded Tabletop Role-Playing System",
        "chapter_count": chapters.len(),
        "chapters": catalog_entries,
    });
    fs::write(
        out_dir.join("catalog.json"),
        serde_json::to_string_pretty(&catalog_json).unwrap(),
    )
    .expect("failed to write docs/book/catalog.json");

    println!(
        "Generated Satya Static Rules Book ({} chapters + index.html + catalog.json) in {}",
        chapters.len(),
        out_dir.display()
    );
}
