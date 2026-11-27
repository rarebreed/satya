//! Unit tests for `satya-rules-book` chapter catalog, search, and Markdown/LaTeX/Mermaid HTML rendering.

use super::*;

#[test]
fn test_rules_book_catalog_search_and_rendering() {
    let chapters = list_rules_chapters();
    assert_eq!(chapters.len(), 24);
    assert!(chapters.iter().any(|c| c.chapter_id == "design_philosophy"));
    assert!(chapters.iter().any(|c| c.chapter_id == "notation_glossary"));
    assert!(chapters.iter().any(|c| c.chapter_id == "tasks"));
    assert!(
        chapters
            .iter()
            .any(|c| c.chapter_id == "task_requirements_catalog")
    );
    assert!(chapters.iter().any(|c| c.chapter_id == "action_economy"));
    assert!(
        chapters
            .iter()
            .any(|c| c.chapter_id == "stealth_and_awareness")
    );
    assert!(chapters.iter().any(|c| c.chapter_id == "skills_and_trees"));
    assert!(
        chapters
            .iter()
            .any(|c| c.chapter_id == "skills_trees_tactical_and_social")
    );
    assert!(
        chapters
            .iter()
            .any(|c| c.chapter_id == "lifepaths_and_economy")
    );
    assert!(
        chapters
            .iter()
            .any(|c| c.chapter_id == "lifepath_events_hobbies_and_factions")
    );
    assert!(chapters.iter().any(|c| c.chapter_id == "psyche_and_social"));
    assert!(chapters.iter().any(|c| c.chapter_id == "tactical_triad"));
    assert!(chapters.iter().any(|c| c.chapter_id == "melee_combat"));
    assert!(
        chapters
            .iter()
            .any(|c| c.chapter_id == "firearms_and_ballistics")
    );
    assert!(
        chapters
            .iter()
            .any(|c| c.chapter_id == "ranged_combat_and_armor")
    );
    assert!(
        chapters
            .iter()
            .any(|c| c.chapter_id == "arsenal_and_explosives")
    );
    assert!(chapters.iter().any(|c| c.chapter_id == "action_economy"));
    assert!(chapters.iter().any(|c| c.chapter_id == "health_and_trauma"));
    assert!(
        chapters
            .iter()
            .any(|c| c.chapter_id == "movement_and_vehicles")
    );
    assert!(
        chapters
            .iter()
            .any(|c| c.chapter_id == "environmental_hazards")
    );
    assert!(
        chapters
            .iter()
            .any(|c| c.chapter_id == "engineering_and_weapon_design")
    );
    assert!(
        chapters
            .iter()
            .any(|c| c.chapter_id == "robotics_and_ai_architecture")
    );
    assert!(
        chapters
            .iter()
            .any(|c| c.chapter_id == "electronic_warfare")
    );

    let tasks_doc = get_rules_chapter("tasks").expect("tasks chapter exists");
    assert!(tasks_doc.markdown.contains("Sarah"));
    assert!(
        tasks_doc
            .rendered_html
            .contains("02-cascading-dice-and-tasks.md"),
        "expected Guided Play Example link to remain intact in rendered HTML"
    );
    assert!(
        !tasks_doc
            .rendered_html
            .contains("02-cascading-dice-and-tasks.md\" data-rules-chapter"),
        "Guided Play Example links must not be rewritten as internal data-rules-chapter links"
    );
    assert!(tasks_doc.rendered_html.contains("satya-rules-article"));
    assert!(
        tasks_doc.rendered_html.contains("rules-math-inline"),
        "expected inline LaTeX math elements in rendered tasks chapter"
    );
    assert!(
        tasks_doc.rendered_html.contains("rules-math-block"),
        "expected display LaTeX math elements in rendered tasks chapter"
    );
    assert!(
        tasks_doc.rendered_html.contains("rules-mermaid"),
        "expected rendered Mermaid flowchart in tasks chapter"
    );
    assert!(
        tasks_doc.rendered_html.contains("mermaid-node"),
        "expected Mermaid flowchart nodes in tasks chapter"
    );
    assert!(
        !tasks_doc.rendered_html.contains(r"\text{TN}_P"),
        "expected raw LaTeX \\text{{TN}}_P to be rendered into HTML math"
    );
    assert!(
        !tasks_doc.rendered_html.contains("S1[\""),
        "expected raw Mermaid node syntax S1[\" to be parsed into flowchart cards"
    );
    for ch in &chapters {
        let doc = get_rules_chapter(&ch.chapter_id).expect("chapter exists");
        assert!(
            !doc.rendered_html.contains("&amp;amp;"),
            "chapter {} should not contain double-escaped &amp;amp;",
            ch.chapter_id
        );
        assert!(
            !doc.rendered_html.contains(r"\text{"),
            "chapter {} should not leak raw \\text{{...}} LaTeX commands",
            ch.chapter_id
        );
        assert!(
            !doc.rendered_html.contains(r"\frac{"),
            "chapter {} should not leak raw \\frac{{...}} LaTeX commands",
            ch.chapter_id
        );
        assert!(
            !doc.rendered_html.contains("!<a"),
            "chapter {} should render Markdown images rather than !<a links",
            ch.chapter_id
        );
    }
    let melee_doc = get_rules_chapter("melee_combat").expect("melee_combat exists");
    assert!(melee_doc.rendered_html.contains("rules-figure"));
    assert!(
        melee_doc
            .rendered_html
            .contains("/docs-assets/07_stairwell_exchange_ooda_triad.jpg")
    );
    let ranged_doc =
        get_rules_chapter("ranged_combat_and_armor").expect("ranged_combat_and_armor exists");
    assert!(ranged_doc.rendered_html.contains("rules-figure"));
    assert!(
        ranged_doc
            .rendered_html
            .contains("/docs-assets/14_civilian_paramilitary_arsenal.jpg")
    );
    // Verify currency strings in lifepaths_and_economy are preserved as literal $ amounts
    let economy_doc =
        get_rules_chapter("lifepaths_and_economy").expect("lifepaths_and_economy exists");
    assert!(economy_doc.rendered_html.contains("$500"));
    // Verify backward-compatible legacy chapter ID aliases
    assert!(get_rules_chapter("satya_d20_rules").is_some());
    assert!(get_rules_chapter("combat").is_some());
    assert!(get_rules_chapter("skill-design").is_some());
    assert!(get_rules_chapter("movement_and_environment").is_some());
    assert_eq!(
        get_rules_chapter("stealth-awareness-and-investigation")
            .expect("stealth alias resolves")
            .summary
            .chapter_id,
        "stealth_and_awareness"
    );
    assert_eq!(
        get_rules_chapter("arsenal-explosives-and-tactical-gear")
            .expect("arsenal alias resolves")
            .summary
            .chapter_id,
        "arsenal_and_explosives"
    );
    assert_eq!(
        get_rules_chapter("weapon-design")
            .expect("weapon-design alias resolves")
            .summary
            .chapter_id,
        "engineering_and_weapon_design"
    );
    assert_eq!(
        get_rules_chapter("cyber-prosthetics")
            .expect("cyber-prosthetics alias resolves")
            .summary
            .chapter_id,
        "robotics_and_ai_architecture"
    );
    assert_eq!(
        get_rules_chapter("ecm-eccm")
            .expect("ecm-eccm alias resolves")
            .summary
            .chapter_id,
        "electronic_warfare"
    );
    assert!(
        get_rules_chapter("engineering_and_weapon_design")
            .expect("engineering doc exists")
            .rendered_html
            .contains("$45,000")
    );
    assert!(
        get_rules_chapter("robotics_and_ai_architecture")
            .expect("robotics doc exists")
            .rendered_html
            .contains("$10,900")
    );
    assert!(
        get_rules_chapter("electronic_warfare")
            .expect("electronic_warfare doc exists")
            .rendered_html
            .contains("$6,500")
    );

    let stealth_hits = search_rules_book("Passive Awareness Alertness Ladder SDR", 3);
    assert!(!stealth_hits.is_empty());
    assert_eq!(stealth_hits[0].chapter_id, "stealth_and_awareness");

    let arsenal_hits = search_rules_book(
        "Linear Shaped Breaching Charge Blast Overpressure Thermite",
        3,
    );
    assert!(!arsenal_hits.is_empty());
    assert_eq!(arsenal_hits[0].chapter_id, "arsenal_and_explosives");

    let eng_hits = search_rules_book("Workshop Facility Custom Firearm Prototype Quirk", 3);
    assert!(!eng_hits.is_empty());
    assert_eq!(eng_hits[0].chapter_id, "engineering_and_weapon_design");

    let robo_hits = search_rules_book(
        "Verification Debt Modular Drone Myomer Cyber-Prosthetics",
        3,
    );
    assert!(!robo_hits.is_empty());
    assert_eq!(robo_hits[0].chapter_id, "robotics_and_ai_architecture");

    let ew_hits = search_rules_book(
        "ECM ECCM Frequency Hopping Triangulation Air-Gapped SCADA",
        3,
    );
    assert!(!ew_hits.is_empty());
    assert_eq!(ew_hits[0].chapter_id, "electronic_warfare");

    let health_hits = search_rules_book("hemorrhaging broken bones blinded catatonia", 5);
    assert!(!health_hits.is_empty());
    assert_eq!(health_hits[0].chapter_id, "health_and_trauma");

    let hits = search_rules_book("Envelope & Fill Tempo Control Force", 5);
    assert!(!hits.is_empty());
    assert!(
        hits[0].chapter_id == "tactical_triad" || hits[0].chapter_id == "melee_combat",
        "expected tactical_triad or melee_combat chapter at top of search hits, got {}",
        hits[0].chapter_id
    );
}

#[test]
fn test_rules_book_ttrpg_layout_directives_and_sidebars() {
    let md = r#"
![Quarter Inset](../../assets/33_whistleblower_crossroads_linnea.jpg#quarter)

![Floated Inset](../../assets/08_zero_hp_parking_deck_triage.jpg#quarter-left)

![Tall Panel](../../assets/07_stairwell_exchange_ooda_triad.jpg#column)

![Half Banner](../../assets/31_ordinary_mortals_analog_table.jpg#half)

![Full Plate](../../assets/23_sarah_lockpicking_cascade.jpg#full)

![Panoramic Spread](../../assets/14_civilian_paramilitary_arsenal.jpg#spread)

> [!SIDEBAR] The Law of Carry
> Every success injects $+6$ into the next pair.
>
> - Bullet one
> - Bullet two
>
> | Stat | Val |
> | :--- | :--- |
> | `C` | `12` |

> [!EXAMPLE] At the Table: Lockpick
> Sarah rolls `4x2d20`.

> [!DOSSIER] Intercept 09-Alpha
> Air-gapped drive recovered.

> [!LORE] Safehouse Notes
> Copper Faraday mesh.

> [!STATBLOCK] Recon Mk-IV
> Standard tactical armor.

<!-- pagebreak -->

<!-- colbreak -->

<!-- raw comment --> <script>alert(1)</script>
"#;
    let html = render_rules_markdown_to_html(md);

    for (hint, img) in [
        ("quarter", "33_whistleblower_crossroads_linnea.jpg"),
        ("quarter-left", "08_zero_hp_parking_deck_triage.jpg"),
        ("column", "07_stairwell_exchange_ooda_triad.jpg"),
        ("half", "31_ordinary_mortals_analog_table.jpg"),
        ("full", "23_sarah_lockpicking_cascade.jpg"),
        ("spread", "14_civilian_paramilitary_arsenal.jpg"),
    ] {
        assert!(
            html.contains(&format!(
                r#"class="rules-figure rules-figure--{hint}" data-layout="{hint}""#
            )),
            "missing figure layout class for #{hint}"
        );
        assert!(
            html.contains(&format!(r#"src="/docs-assets/{img}""#)),
            "layout fragment #{hint} should be stripped from img src for {img}"
        );
        assert!(!html.contains(&format!("{img}#{hint}")));
    }

    assert!(html.contains(r#"<aside class="rules-alert rules-sidebar">"#));
    assert!(html.contains(r#"<div class="rules-alert-title">The Law of Carry</div>"#));
    assert!(html.contains("<li>Bullet one</li>"));
    assert!(html.contains("<table class=\"rules-table\">"));
    assert!(html.contains(r#"<aside class="rules-alert rules-example">"#));
    assert!(html.contains(r#"<div class="rules-alert-title">At the Table: Lockpick</div>"#));
    assert!(html.contains(r#"<aside class="rules-alert rules-dossier">"#));
    assert!(html.contains(r#"<div class="rules-alert-title">Intercept 09-Alpha</div>"#));
    assert!(html.contains(r#"<div class="rules-alert-title">Safehouse Notes</div>"#));
    assert!(html.contains(r#"<aside class="rules-alert rules-statblock">"#));
    assert!(html.contains(r#"<div class="rules-alert-title">Recon Mk-IV</div>"#));

    assert!(
        html.contains(
            r#"<div class="rules-page-break" data-break="page" aria-hidden="true"></div>"#
        )
    );
    assert!(
        html.contains(
            r#"<div class="rules-col-break" data-break="column" aria-hidden="true"></div>"#
        )
    );

    assert!(!html.contains("<script>"));
    assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
    assert!(html.contains("&lt;!-- raw comment --&gt;"));
}

#[test]
fn test_markdown_url_sanitization_blocks_javascript_and_data_uris() {
    let md = concat!(
        "[bad](javascript:alert(1)) ",
        "[bad2](DATA:text/html,<script>alert(1)</script>) ",
        "![img](javascript:alert(2)) ",
        "[ok-ext](https://example.com/rules?a=1&b=2) ",
        "[ok-anchor](#combat-basics)"
    );
    let html = crate::rules_book_html::render_rules_markdown_to_html(md);
    assert!(!html.contains("javascript:"), "unsafe javascript: URI leaked: {html}");
    assert!(!html.contains("data:text/html"), "unsafe data: URI leaked: {html}");
    assert!(!html.contains("DATA:text/html"), "unsafe DATA: URI leaked: {html}");
    assert!(
        html.contains(r#"href="https://example.com/rules?a=1&amp;b=2" target="_blank" rel="noopener noreferrer""#),
        "external https link should retain rel=noopener noreferrer: {html}"
    );
    assert!(
        html.contains(r##"href="#combat-basics""##),
        "relative anchor link should be preserved: {html}"
    );
}

#[test]
fn mermaid_renders_svg_flowchart_with_arrows_and_no_raw_newlines() {
    let src = concat!(
        "flowchart TD\n",
        "    A[\"First Line\\n(Second Line)\"] --> B[\"Target Title\\n(Extra Detail)\"]\n",
        "    A -->|\"Labeled\"| C[\"Third Node\\n(More Detail)\"]\n",
        "    C --> A\n",
    );
    let html = render_mermaid_to_html(src);
    assert!(
        html.contains(r#"<svg class="mermaid-svg""#),
        "expected inline SVG flowchart element: {html}"
    );
    assert!(
        html.contains(r#"<g class="mermaid-node">"#),
        "expected SVG node groups: {html}"
    );
    assert!(
        html.contains(r#"<path class="mermaid-edge""#),
        "expected SVG edge paths: {html}"
    );
    assert!(
        html.contains(r#"<marker id="m-arr-"#),
        "expected SVG arrowhead marker definition: {html}"
    );
    assert!(
        html.contains("First Line</tspan>"),
        "expected wrapped/split tspan lines: {html}"
    );
    assert!(
        html.contains("mermaid-edge--back"),
        "expected cycle edge to be routed as back-edge: {html}"
    );
    assert!(
        html.contains("Labeled"),
        "expected edge label pill text: {html}"
    );
    assert!(
        !html.contains(r"\n"),
        "literal \\n must not leak into rendered Mermaid SVG: {html}"
    );

    let sg_src = concat!(
        "flowchart TD\n",
        "    PRO[\"Prologue\"] --> S1\n",
        "    subgraph S1[\"Group One\"]\n",
        "        N1[\"Node One\"] --> N2[\"Node Two\"]\n",
        "    end\n",
    );
    let sg_html = render_mermaid_to_html(sg_src);
    assert!(
        sg_html.contains(r#"<g class="mermaid-subgraph">"#),
        "expected subgraph SVG group: {sg_html}"
    );
}
