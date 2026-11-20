//! CLI utility (`cargo run -p satya-engine --bin build-rules-book`) that compiles all 24 canonical
//! chapters of the Satya TTRPG Rules Book (`docs/ttrpg/`) into static HTML pages and `catalog.json`
//! inside `docs/book/`.

use satya_engine::{
    BotchRule, CarryMode, MarginAggregation, canonical_rules_chapters, compute_stats,
    escape_html_text, paired_list_signed_margin_pmfs, render_rules_markdown_to_html,
};
use serde_json::json;
use std::fs;
use std::path::Path;

const SATYA_BOOK_CSS: &str = include_str!("../../assets/rules_book/satya_book.css");
const SATYA_BOOK_JS: &str = include_str!("../../assets/rules_book/satya_book.js");

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
            "02-core-mechanics/stealth_awareness_and_investigation.md",
            "stealth_and_awareness.html",
        ),
        (
            "../02-core-mechanics/stealth_awareness_and_investigation.md",
            "stealth_and_awareness.html",
        ),
        (
            "stealth_awareness_and_investigation.md",
            "stealth_and_awareness.html",
        ),
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
            "03-character/lifepath_events_hobbies_and_factions.md",
            "lifepath_events_hobbies_and_factions.html",
        ),
        (
            "../03-character/lifepath_events_hobbies_and_factions.md",
            "lifepath_events_hobbies_and_factions.html",
        ),
        (
            "lifepath_events_hobbies_and_factions.md",
            "lifepath_events_hobbies_and_factions.html",
        ),
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
        ("firearms_and_ballistics.md", "firearms_and_ballistics.html"),
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
            "05-combat/arsenal_explosives_and_tactical_gear.md",
            "arsenal_and_explosives.html",
        ),
        (
            "../05-combat/arsenal_explosives_and_tactical_gear.md",
            "arsenal_and_explosives.html",
        ),
        (
            "arsenal_explosives_and_tactical_gear.md",
            "arsenal_and_explosives.html",
        ),
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
            "06-health-and-survival/movement_encumbrance_and_vehicles.md",
            "movement_and_vehicles.html",
        ),
        (
            "../06-health-and-survival/movement_encumbrance_and_vehicles.md",
            "movement_and_vehicles.html",
        ),
        (
            "movement_encumbrance_and_vehicles.md",
            "movement_and_vehicles.html",
        ),
        (
            "06-health-and-survival/environmental_hazards_and_survival.md",
            "environmental_hazards.html",
        ),
        (
            "../06-health-and-survival/environmental_hazards_and_survival.md",
            "environmental_hazards.html",
        ),
        (
            "environmental_hazards_and_survival.md",
            "environmental_hazards.html",
        ),
        (
            "06-health-and-survival/movement_vehicles_and_environment.md",
            "movement_and_vehicles.html",
        ),
        (
            "../06-health-and-survival/movement_vehicles_and_environment.md",
            "movement_and_vehicles.html",
        ),
        (
            "movement_vehicles_and_environment.md",
            "movement_and_vehicles.html",
        ),
        (
            "07-engineering-and-tech/engineering_and_weapon_design.md",
            "engineering_and_weapon_design.html",
        ),
        (
            "../07-engineering-and-tech/engineering_and_weapon_design.md",
            "engineering_and_weapon_design.html",
        ),
        (
            "engineering_and_weapon_design.md",
            "engineering_and_weapon_design.html",
        ),
        (
            "07-engineering-and-tech/robotics_drones_and_ai_architecture.md",
            "robotics_and_ai_architecture.html",
        ),
        (
            "../07-engineering-and-tech/robotics_drones_and_ai_architecture.md",
            "robotics_and_ai_architecture.html",
        ),
        (
            "robotics_drones_and_ai_architecture.md",
            "robotics_and_ai_architecture.html",
        ),
        (
            "07-engineering-and-tech/electronic_warfare_and_surveillance.md",
            "electronic_warfare.html",
        ),
        (
            "../07-engineering-and-tech/electronic_warfare_and_surveillance.md",
            "electronic_warfare.html",
        ),
        (
            "electronic_warfare_and_surveillance.md",
            "electronic_warfare.html",
        ),
    ];
    let mut out = rendered_html.to_string();
    for (src, dst) in mapping {
        out = out.replace(&format!("href=\"{src}\""), &format!("href=\"{dst}\""));
        out = out.replace(&format!("href=\"{src}#"), &format!("href=\"{dst}#"));
    }
    out.replace("href=\"../../ttrpg-examples/", "href=\"../ttrpg-examples/")
        .replace("href=\"../../campaign/", "href=\"../campaign/")
        .replace("src=\"/docs-assets/", "src=\"../assets/")
        .replace("src=\"../../assets/", "src=\"../assets/")
}

fn part_index_from_label(part: &str) -> u8 {
    if part.starts_with("Part VII") {
        7
    } else if part.starts_with("Part VI") {
        6
    } else if part.starts_with("Part V") {
        5
    } else if part.starts_with("Part IV") {
        4
    } else if part.starts_with("Part III") {
        3
    } else if part.starts_with("Part II") {
        2
    } else {
        1
    }
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

    let (chapter_num, part_label) = chapters
        .iter()
        .enumerate()
        .find(|(_, d)| d.summary.chapter_id == active_chapter_id)
        .map(|(i, d)| (i + 1, d.summary.part.as_str()))
        .unwrap_or((0, "Part I-VII: Reference & Sandbox"));
    let part_index = part_index_from_label(part_label);
    let part_short = part_label.split(':').next().unwrap_or(part_label).trim();
    let esc_title = escape_html_text(title);
    let esc_part = escape_html_text(part_label);
    let esc_part_short = escape_html_text(part_short);
    let esc_cid = escape_html_text(active_chapter_id);

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8"/>
  <meta name="viewport" content="width=device-width, initial-scale=1"/>
  <title>{esc_title} — Satya TTRPG Rules Book</title>
  <link rel="stylesheet" href="satya_book.css"/>
  <script defer src="satya_book.js"></script>
</head>
<body
  data-book-theme="sangha-robes"
  data-book-view="spread"
  data-part-index="{part_index}"
  data-part-label="{esc_part}"
  data-chapter-num="{chapter_num}"
  data-chapter-title="{esc_title}"
  data-chapter-id="{esc_cid}"
>
  <header class="book-toolbar">
    <div class="toolbar-cluster">
      <button id="btn-toc-toggle" type="button" class="toolbar-btn" title="Toggle Chapters (T)">&#9776; Chapters</button>
      <a href="index.html" class="toolbar-brand">SATYA // RULES BOOK</a>
      <span class="toolbar-crumb">{esc_part_short} &middot; {esc_title}</span>
    </div>
    <div class="toolbar-cluster">
      <div class="toolbar-group" role="group" aria-label="Book Theme">
        <button type="button" class="toolbar-btn is-active" data-set-theme="sangha-robes" title="Korean Seon grey, Japanese Zen black &amp; Vietnamese Thiền saffron robes">Sangha Robes</button>
        <button type="button" class="toolbar-btn" data-set-theme="chan-sanctuary">Chan Wood</button>
        <button type="button" class="toolbar-btn" data-set-theme="tactical-hud">Tactical HUD</button>
      </div>
      <div class="toolbar-group" role="group" aria-label="Page View">
        <button type="button" class="toolbar-btn is-active" data-set-view="spread">2-Page Spread</button>
        <button type="button" class="toolbar-btn" data-set-view="single">Single Page</button>
        <button type="button" class="toolbar-btn" data-set-view="scroll">Scroll</button>
      </div>
    </div>
    <div class="toolbar-cluster">
      <button id="btn-page-prev" type="button" class="toolbar-btn" title="Previous Page (Left Arrow)">&larr; Prev</button>
      <label class="page-jump-wrap" aria-label="Jump to Page">
        <span>Pg</span>
        <input id="book-page-input" type="number" min="1" value="1"/>
        <span id="book-page-total">/ 1</span>
      </label>
      <button id="btn-page-next" type="button" class="toolbar-btn" title="Next Page (Right Arrow)">Next &rarr;</button>
      <button id="btn-book-print" type="button" class="toolbar-btn" title="Print or Save PDF">Print / PDF</button>
    </div>
  </header>
  <div class="book-shell">
    <aside id="book-sidebar" class="book-sidebar">
      <div class="book-brand">SATYA // RULES BOOK</div>
      <div class="book-sub">2d20 Watershed Cascade &amp; Tactical Triad</div>
      {nav_items}
    </aside>
    <main class="book-workspace">
      <div id="book-source" class="book-source-content">
        {body_html}
      </div>
      <div id="book-stage" class="book-stage" aria-live="polite"></div>
      <div class="book-pager">
        {footer_links}
      </div>
    </main>
  </div>
</body>
</html>"#
    )
}

fn main() {
    let out_dir = Path::new("docs").join("book");
    fs::create_dir_all(&out_dir).expect("failed to create docs/book directory");
    fs::write(out_dir.join("satya_book.css"), SATYA_BOOK_CSS)
        .expect("failed to write docs/book/satya_book.css");
    fs::write(out_dir.join("satya_book.js"), SATYA_BOOK_JS)
        .expect("failed to write docs/book/satya_book.js");

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
