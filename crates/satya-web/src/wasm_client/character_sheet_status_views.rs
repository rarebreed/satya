//! Pure HTML formatters for the Character Sheet's **Human-Shaped Anatomical Hit Location Diagram**,
//! **Damage / Fatigue / Psyche Status Trackers**, and **Writable Operator Field Notes & Lifepath Dossier**.

use crate::wasm_client::character_sheet::{
    AnatomicalZoneState, BoneFractureStatus, CharacterSheetState,
};
use crate::wasm_client::dom::escape_html;
use satya_engine::{
    AcuteMentalTraumaTier, HIT_LOCATION_TABLE, HemorrhageGrade, HypovolemicShockClass, MoraleState,
    PhysiologicalStatusCondition, SocialWoundKind, evaluate_blood_loss_progression,
    evaluate_fatigue_level, evaluate_operational_heat, lookup_campaign_sqm, psyche_drift_threshold,
};

fn zone_colors(z: &AnatomicalZoneState, is_selected: bool) -> (&'static str, &'static str) {
    let severe = z.wound_tier >= 3
        || matches!(
            z.bone_status,
            BoneFractureStatus::CompleteFracture | BoneFractureStatus::CompoundShattered
        )
        || matches!(
            z.bleed_grade,
            HemorrhageGrade::Grade3MajorVenousVisceral | HemorrhageGrade::Grade4ArterialSeverance
        );
    let moderate = z.is_injured();
    let fill = if severe {
        "rgba(244, 63, 94, 0.42)"
    } else if moderate {
        "rgba(245, 158, 11, 0.36)"
    } else if is_selected {
        "rgba(56, 189, 248, 0.28)"
    } else {
        "rgba(15, 23, 42, 0.85)"
    };
    let stroke = if is_selected {
        "#f8fafc"
    } else if severe {
        "#f43f5e"
    } else if moderate {
        "#f59e0b"
    } else {
        "#38bdf8"
    };
    (fill, stroke)
}

fn bleed_short_label(g: HemorrhageGrade) -> &'static str {
    match g {
        HemorrhageGrade::Grade0None => "No Bleed",
        HemorrhageGrade::Grade1Capillary => "G1 Capillary (0.25 U/hr)",
        HemorrhageGrade::Grade2Venous => "G2 Venous (0.5 U/min)",
        HemorrhageGrade::Grade3MajorVenousVisceral => "G3 Major Venous (1 U/5s)",
        HemorrhageGrade::Grade4ArterialSeverance => "G4 Arterial (1 U/1s)",
    }
}

fn bleed_grade_num(g: HemorrhageGrade) -> u8 {
    match g {
        HemorrhageGrade::Grade0None => 0,
        HemorrhageGrade::Grade1Capillary => 1,
        HemorrhageGrade::Grade2Venous => 2,
        HemorrhageGrade::Grade3MajorVenousVisceral => 3,
        HemorrhageGrade::Grade4ArterialSeverance => 4,
    }
}

/// Renders an interactive human-shaped anatomical SVG silhouette (`viewBox="0 0 320 430"`)
/// with all 13 canonical `1d20` hit zones, fracture/bleed status callouts, and click-to-select handlers.
#[must_use]
pub fn format_human_hit_location_svg(sheet: &CharacterSheetState) -> String {
    let sel = sheet.selected_hit_zone.min(12);
    let c = |idx: usize| zone_colors(&sheet.hit_zones[idx], sel == idx);

    let (f_head, s_head) = c(12);
    let (f_neck, s_neck) = c(11);
    let (f_shld, s_shld) = c(8);
    let (f_chest, s_chest) = c(6);
    let (f_vital, s_vital) = c(7);
    let (f_arm, s_arm) = c(9);
    let (f_hand, s_hand) = c(10);
    let (f_abd, s_abd) = c(5);
    let (f_pelv, s_pelv) = c(4);
    let (f_thigh, s_thigh) = c(3);
    let (f_knee, s_knee) = c(2);
    let (f_calf, s_calf) = c(1);
    let (f_foot, s_foot) = c(0);

    // Collect active injury callouts to display along the right margin of the SVG
    let mut callouts = Vec::new();
    for (idx, z) in sheet.hit_zones.iter().enumerate() {
        if z.is_injured() {
            let spec = &HIT_LOCATION_TABLE[idx];
            let mut tags = Vec::new();
            if z.wound_tier > 0 {
                tags.push(format!("T{}", z.wound_tier));
            }
            if z.bone_status != BoneFractureStatus::Intact {
                tags.push(z.bone_status.short_badge().to_string());
            }
            if z.bleed_grade != HemorrhageGrade::Grade0None {
                tags.push(format!("Bleed G{}", bleed_grade_num(z.bleed_grade)));
            }
            callouts.push(format!(
                "[d20 {}] {}: {}",
                if spec.d20_min == spec.d20_max {
                    format!("{}", spec.d20_min)
                } else {
                    format!("{}-{}", spec.d20_min, spec.d20_max)
                },
                spec.name.split(" (").next().unwrap_or(spec.name),
                tags.join(" · ")
            ));
        }
    }
    let callout_svg: String = if callouts.is_empty() {
        r##"<text x="160" y="418" text-anchor="middle" fill="#34d399" font-family="monospace" font-size="10">ALL 13 ANATOMICAL ZONES INTACT · ZERO ACTIVE BLEEDS</text>"##.to_string()
    } else {
        let summary = callouts.join(" | ");
        format!(
            r##"<text x="160" y="418" text-anchor="middle" fill="#f43f5e" font-family="monospace" font-size="9.5">{}</text>"##,
            escape_html(&summary)
        )
    };

    format!(
        r##"<svg id="char-human-hit-svg" viewBox="0 0 320 430" class="char-human-svg" role="img" aria-label="Interactive Human Anatomical Hit Location Status Diagram">
            <rect width="320" height="430" rx="8" fill="#070b12"/>
            <line x1="160" y1="12" x2="160" y2="395" stroke="#1e293b" stroke-width="1" stroke-dasharray="3 3"/>
            <text x="12" y="20" fill="#94a3b8" font-family="monospace" font-size="10">1d20 ANATOMICAL SILHOUETTE</text>
            <text x="12" y="34" fill="#64748b" font-family="monospace" font-size="9">Click any body region to inspect/edit</text>

            <!-- 12: Head (d20 = 20) -->
            <g class="hit-zone-group" onclick="window.satyaCharSheetSelectZone(12)">
                <title>20: Head (Cranium / Face / Jaw / Eyes)</title>
                <ellipse cx="160" cy="46" rx="22" ry="26" fill="{f_head}" stroke="{s_head}" stroke-width="2"/>
                <text x="160" y="50" text-anchor="middle" fill="#e2e8f0" font-family="monospace" font-size="9">20:HEAD</text>
            </g>

            <!-- 11: Neck / Throat / Carotid (d20 = 19) -->
            <g class="hit-zone-group" onclick="window.satyaCharSheetSelectZone(11)">
                <title>19: Neck / Throat / Carotid / Cervical</title>
                <rect x="148" y="73" width="24" height="16" rx="4" fill="{f_neck}" stroke="{s_neck}" stroke-width="1.8"/>
                <text x="160" y="84" text-anchor="middle" fill="#e2e8f0" font-family="monospace" font-size="8">19:NECK</text>
            </g>

            <!-- 8: Shoulders / Clavicle / Scapula (d20 = 15) -->
            <g class="hit-zone-group" onclick="window.satyaCharSheetSelectZone(8)">
                <title>15: Shoulder / Clavicle / Scapula (L or R)</title>
                <polygon points="104,92 146,90 142,112 100,114" fill="{f_shld}" stroke="{s_shld}" stroke-width="1.8"/>
                <polygon points="174,90 216,92 220,114 178,112" fill="{f_shld}" stroke="{s_shld}" stroke-width="1.8"/>
                <text x="123" y="105" text-anchor="middle" fill="#cbd5e1" font-family="monospace" font-size="7.5">15:SHLD</text>
                <text x="197" y="105" text-anchor="middle" fill="#cbd5e1" font-family="monospace" font-size="7.5">15:SHLD</text>
            </g>

            <!-- 6: Chest / Ribcage / Lungs (d20 = 11-13) -->
            <g class="hit-zone-group" onclick="window.satyaCharSheetSelectZone(6)">
                <title>11-13: Chest / Ribcage / Lungs</title>
                <polygon points="124,92 196,92 190,152 130,152" fill="{f_chest}" stroke="{s_chest}" stroke-width="2"/>
                <text x="160" y="108" text-anchor="middle" fill="#f8fafc" font-family="monospace" font-size="8.5">11-13:CHEST</text>
            </g>

            <!-- 7: Vital Thoracic Heart / Spine / Aorta (d20 = 14) -->
            <g class="hit-zone-group" onclick="window.satyaCharSheetSelectZone(7)">
                <title>14: Vital Thoracic (Heart / Spine / Aorta)</title>
                <polygon points="160,114 175,128 160,145 145,128" fill="{f_vital}" stroke="{s_vital}" stroke-width="2"/>
                <text x="160" y="132" text-anchor="middle" fill="#fda4af" font-family="monospace" font-size="7.5">14:VITAL</text>
            </g>

            <!-- 9: Upper Arm / Elbow / Forearm (d20 = 16-17) -->
            <g class="hit-zone-group" onclick="window.satyaCharSheetSelectZone(9)">
                <title>16-17: Upper Arm / Elbow / Forearm (L or R)</title>
                <rect x="88" y="116" width="24" height="78" rx="8" fill="{f_arm}" stroke="{s_arm}" stroke-width="1.8"/>
                <rect x="208" y="116" width="24" height="78" rx="8" fill="{f_arm}" stroke="{s_arm}" stroke-width="1.8"/>
                <text x="100" y="156" text-anchor="middle" fill="#cbd5e1" font-family="monospace" font-size="7.5">16-17</text>
                <text x="220" y="156" text-anchor="middle" fill="#cbd5e1" font-family="monospace" font-size="7.5">ARM</text>
            </g>

            <!-- 10: Wrist / Hand / Fingers (d20 = 18) -->
            <g class="hit-zone-group" onclick="window.satyaCharSheetSelectZone(10)">
                <title>18: Wrist / Hand / Fingers (L or R)</title>
                <rect x="86" y="198" width="26" height="26" rx="6" fill="{f_hand}" stroke="{s_hand}" stroke-width="1.8"/>
                <rect x="208" y="198" width="26" height="26" rx="6" fill="{f_hand}" stroke="{s_hand}" stroke-width="1.8"/>
                <text x="99" y="214" text-anchor="middle" fill="#cbd5e1" font-family="monospace" font-size="7.5">18:HND</text>
                <text x="221" y="214" text-anchor="middle" fill="#cbd5e1" font-family="monospace" font-size="7.5">18:HND</text>
            </g>

            <!-- 5: Abdomen / Gut / Liver / Spleen (d20 = 8-10) -->
            <g class="hit-zone-group" onclick="window.satyaCharSheetSelectZone(5)">
                <title>8-10: Abdomen / Gut / Liver / Spleen</title>
                <polygon points="130,155 190,155 186,196 134,196" fill="{f_abd}" stroke="{s_abd}" stroke-width="2"/>
                <text x="160" y="179" text-anchor="middle" fill="#f8fafc" font-family="monospace" font-size="8.5">8-10:ABDOMEN</text>
            </g>

            <!-- 4: Pelvis / Hip Girdle / Groin (d20 = 7) -->
            <g class="hit-zone-group" onclick="window.satyaCharSheetSelectZone(4)">
                <title>7: Pelvis / Hip Girdle / Groin</title>
                <polygon points="128,199 192,199 184,228 136,228" fill="{f_pelv}" stroke="{s_pelv}" stroke-width="2"/>
                <text x="160" y="217" text-anchor="middle" fill="#f8fafc" font-family="monospace" font-size="8.5">7:PELVIS</text>
            </g>

            <!-- 3: Thigh / Femur / Femoral (d20 = 5-6) -->
            <g class="hit-zone-group" onclick="window.satyaCharSheetSelectZone(3)">
                <title>5-6: Thigh (Femur / Femoral Artery)</title>
                <rect x="124" y="232" width="30" height="62" rx="8" fill="{f_thigh}" stroke="{s_thigh}" stroke-width="2"/>
                <rect x="166" y="232" width="30" height="62" rx="8" fill="{f_thigh}" stroke="{s_thigh}" stroke-width="2"/>
                <text x="139" y="266" text-anchor="middle" fill="#f8fafc" font-family="monospace" font-size="8">5-6:THG</text>
                <text x="181" y="266" text-anchor="middle" fill="#f8fafc" font-family="monospace" font-size="8">5-6:THG</text>
            </g>

            <!-- 2: Knee Joint (Patella/ACL) (d20 = 4) -->
            <g class="hit-zone-group" onclick="window.satyaCharSheetSelectZone(2)">
                <title>4: Knee Joint (Patella / ACL)</title>
                <rect x="126" y="297" width="26" height="22" rx="6" fill="{f_knee}" stroke="{s_knee}" stroke-width="1.8"/>
                <rect x="168" y="297" width="26" height="22" rx="6" fill="{f_knee}" stroke="{s_knee}" stroke-width="1.8"/>
                <text x="139" y="311" text-anchor="middle" fill="#cbd5e1" font-family="monospace" font-size="7.5">4:KNEE</text>
                <text x="181" y="311" text-anchor="middle" fill="#cbd5e1" font-family="monospace" font-size="7.5">4:KNEE</text>
            </g>

            <!-- 1: Calf / Shin (Tibia/Fibula) (d20 = 2-3) -->
            <g class="hit-zone-group" onclick="window.satyaCharSheetSelectZone(1)">
                <title>2-3: Calf / Shin (Tibia / Fibula)</title>
                <rect x="127" y="322" width="24" height="52" rx="6" fill="{f_calf}" stroke="{s_calf}" stroke-width="1.8"/>
                <rect x="169" y="322" width="24" height="52" rx="6" fill="{f_calf}" stroke="{s_calf}" stroke-width="1.8"/>
                <text x="139" y="351" text-anchor="middle" fill="#cbd5e1" font-family="monospace" font-size="7.5">2-3:SHN</text>
                <text x="181" y="351" text-anchor="middle" fill="#cbd5e1" font-family="monospace" font-size="7.5">2-3:SHN</text>
            </g>

            <!-- 0: Foot / Ankle (d20 = 1) -->
            <g class="hit-zone-group" onclick="window.satyaCharSheetSelectZone(0)">
                <title>1: Foot / Ankle (L or R)</title>
                <polygon points="118,377 152,377 154,398 112,398" fill="{f_foot}" stroke="{s_foot}" stroke-width="1.8"/>
                <polygon points="168,377 202,377 208,398 166,398" fill="{f_foot}" stroke="{s_foot}" stroke-width="1.8"/>
                <text x="134" y="391" text-anchor="middle" fill="#cbd5e1" font-family="monospace" font-size="7.5">1:FOOT</text>
                <text x="186" y="391" text-anchor="middle" fill="#cbd5e1" font-family="monospace" font-size="7.5">1:FOOT</text>
            </g>

            {callout_svg}
        </svg>"##
    )
}

/// Formats the Status Tracker (Damage, Fatigue, Psyche, and Human-Shaped Hit Locations) section.
#[must_use]
pub fn format_character_status_and_hit_locations_html(sheet: &CharacterSheetState) -> String {
    let c = &sheet.characteristics;
    let derived = c.compute_derived(0);
    let fat_lvl = sheet.effective_fatigue_level();
    let fat_spec = evaluate_fatigue_level(fat_lvl);
    let shock_idx = sheet.effective_wound_shock_index();
    let active_bleeds = sheet.active_bleeds();
    let blood_eval = evaluate_blood_loss_progression(sheet.blood_units_lost, &active_bleeds, 0);
    let wound_caps = sheet.compute_anatomical_wound_cap_penalties();
    let delta_int = sheet.total_task_delta_int();

    let sel_idx = sheet.selected_hit_zone.min(12);
    let sel_zone = &sheet.hit_zones[sel_idx];
    let sel_spec = &HIT_LOCATION_TABLE[sel_idx];

    let wound_tier_opts: String = [
        (0u8, "Tier 0: Uninjured (0 Cap Pen)"),
        (1, "Tier 1: Superficial / Minor (-2 Cap, 3–7d)"),
        (2, "Tier 2: Moderate Trauma (-5 Cap, +1 Shock, 2–4w)"),
        (3, "Tier 3: Severe / Fracture (-10 Cap, +2 Shock, 6–12w)"),
        (4, "Tier 4: Critical / Maiming (Cap Locked 0, +3 Shock)"),
        (5, "Tier 5: Catastrophic / Lethal (Cap Locked 0, +4 Shock)"),
    ]
    .iter()
    .map(|&(t, lbl)| {
        let s = if sel_zone.wound_tier == t {
            " selected"
        } else {
            ""
        };
        format!(r#"<option value="{t}"{s}>{lbl}</option>"#)
    })
    .collect::<Vec<_>>()
    .join("");

    let bone_opts: String = BoneFractureStatus::ALL
        .iter()
        .map(|&b| {
            let s = if sel_zone.bone_status == b {
                " selected"
            } else {
                ""
            };
            format!(r#"<option value="{}"{s}>{}</option>"#, b.code(), b.label())
        })
        .collect::<Vec<_>>()
        .join("");

    let cur_bleed_num = bleed_grade_num(sel_zone.bleed_grade);
    let bleed_opts: String = [
        (0u8, "Grade 0: No Active Bleed"),
        (1, "Grade 1: Capillary Ooze (0.25 U/hr · 8t Bandage)"),
        (2, "Grade 2: Venous Hemorrhage (0.5 U/min · 20t Dressing)"),
        (3, "Grade 3: Major Venous / Visceral (1.0 U/5s · 20t Pack)"),
        (
            4,
            "Grade 4: Arterial Severance (1.0 U/1s · 24t CAT Tourniquet)",
        ),
    ]
    .iter()
    .map(|&(g, lbl)| {
        let s = if cur_bleed_num == g { " selected" } else { "" };
        format!(r#"<option value="{g}"{s}>{lbl}</option>"#)
    })
    .collect::<Vec<_>>()
    .join("");

    let hit_table_rows: String = HIT_LOCATION_TABLE
        .iter()
        .enumerate()
        .map(|(idx, spec)| {
            let z = &sheet.hit_zones[idx];
            let d20_str = if spec.d20_min == spec.d20_max {
                format!("{}", spec.d20_min)
            } else {
                format!("{}–{}", spec.d20_min, spec.d20_max)
            };
            let mut slashed = Vec::new();
            if spec.slashes_tempo_cap {
                slashed.push("Tempo");
            }
            if spec.slashes_control_cap {
                slashed.push("Control");
            }
            if spec.slashes_force_cap {
                slashed.push("Force");
            }
            let w_badge = if z.wound_tier == 0 {
                r#"<span class="pill pill-muted">T0 Healthy</span>"#.to_string()
            } else {
                format!(
                    r#"<span class="pill pill-danger">Tier {} (-{} Cap)</span>"#,
                    z.wound_tier,
                    z.effective_cap_penalty()
                )
            };
            let b_badge = if z.bone_status == BoneFractureStatus::Intact {
                r#"<span class="stat-note">Intact</span>"#.to_string()
            } else {
                format!(
                    r#"<span class="pill pill-danger">{}</span>"#,
                    z.bone_status.short_badge()
                )
            };
            let bl_badge = if z.bleed_grade == HemorrhageGrade::Grade0None {
                r#"<span class="stat-note">None</span>"#.to_string()
            } else {
                format!(
                    r#"<span class="pill pill-danger">{}</span>"#,
                    bleed_short_label(z.bleed_grade)
                )
            };
            let row_cls = if idx == sel_idx {
                r#" class="char-row-selected""#
            } else {
                ""
            };
            format!(
                r#"<tr{row_cls} onclick="window.satyaCharSheetSelectZone({idx})">
                    <td class="mono"><strong>{d20_str}</strong> <span class="stat-note">(+{ext} Δ_ext)</span></td>
                    <td><strong>{name}</strong></td>
                    <td><span class="stat-note">{slashed_str}</span></td>
                    <td>{w_badge}</td>
                    <td>{b_badge}</td>
                    <td>{bl_badge}</td>
                </tr>"#,
                ext = spec.called_shot_delta_ext,
                name = escape_html(spec.name),
                slashed_str = slashed.join("/"),
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let hypovolemic_lbl = match blood_eval.hypovolemic_class {
        HypovolemicShockClass::None => "Euvolemic / Compensated (<1.0 U lost)",
        HypovolemicShockClass::ClassIMild => "Class I Mild Shock (1.0–1.99 U · +1 Fatigue)",
        HypovolemicShockClass::ClassIIModerate => {
            "Class II Moderate Shock (2.0–3.49 U · +1 Shock, +2 Fatigue, Dizzy)"
        }
        HypovolemicShockClass::ClassIIISevere => {
            "Class III Severe Shock (3.5–4.99 U · +3 Shock, +4 Fatigue, WSR/20t)"
        }
        HypovolemicShockClass::ClassIVLethalComa => {
            "Class IV Lethal Coma (5.0+ U · Unconscious, Arrest in 240t)"
        }
    };

    let status_cond_specs = [
        (
            "DAZZLED",
            "Dazzled (-5 SIG)",
            PhysiologicalStatusCondition::DazzledBlurredVision,
        ),
        (
            "BLINDED",
            "Blinded (SIG=0)",
            PhysiologicalStatusCondition::Blinded,
        ),
        (
            "TINNITUS",
            "Tinnitus (-6 HEA)",
            PhysiologicalStatusCondition::Tinnitus,
        ),
        (
            "DEAFENED",
            "Deafened (HEA=0)",
            PhysiologicalStatusCondition::Deafened,
        ),
        (
            "STUNNED",
            "Stunned (+8t Freeze)",
            PhysiologicalStatusCondition::StunnedDazed,
        ),
        (
            "CONCUSSED",
            "Concussed (-4 Cog)",
            PhysiologicalStatusCondition::Concussed,
        ),
        (
            "DIZZY",
            "Dizzy (-6 KIN)",
            PhysiologicalStatusCondition::DizzyVertigo,
        ),
        (
            "NAUSEA",
            "Nausea (+4 Δ_int)",
            PhysiologicalStatusCondition::NauseaSickened,
        ),
        (
            "PINNED",
            "Pinned (Tempo=0)",
            PhysiologicalStatusCondition::PinnedImmobilized,
        ),
    ];
    let cond_buttons: String = status_cond_specs
        .iter()
        .map(|&(code, lbl, cond)| {
            let active = sheet.active_status_conditions.contains(&cond);
            let cls = if active { "btn-chip active-cond" } else { "btn-chip" };
            format!(
                r#"<button type="button" class="{cls}" onclick="window.satyaCharSheetUpdateStatus('toggle_condition', '{code}')">{lbl}</button>"#
            )
        })
        .collect::<Vec<_>>()
        .join(" ");

    let morale_opts: String = MoraleState::ALL
        .iter()
        .map(|&m| {
            let s = if sheet.morale_state == m {
                " selected"
            } else {
                ""
            };
            format!(
                r#"<option value="{}"{s}>{} ({:+} Pool)</option>"#,
                m.as_str(),
                m.label(),
                m.pool_modifier()
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let cur_mt = match sheet.mental_trauma {
        Some(AcuteMentalTraumaTier::Tier1AcuteMentalShockOodaFreeze) => "TIER_1",
        Some(AcuteMentalTraumaTier::Tier2PanicFightOrFlight) => "TIER_2",
        Some(AcuteMentalTraumaTier::Tier3DissociationShellShock) => "TIER_3",
        Some(AcuteMentalTraumaTier::Tier4Catatonia) => "TIER_4",
        None => "NONE",
    };
    let mt_opts: String = [
        ("NONE", "None (Mentally Composed)"),
        ("TIER_1", "Tier 1: Acute Mental Shock (OODA Freeze 9t)"),
        (
            "TIER_2",
            "Tier 2: Panic / Fight-or-Flight (-4 Pool, -6 ANA/FOC)",
        ),
        (
            "TIER_3",
            "Tier 3: Dissociation / Shell Shock (-6 ITU/INS/FOC, +0.30x Time)",
        ),
        ("TIER_4", "Tier 4: Catatonia (Total Psychomotor Shutdown)"),
    ]
    .iter()
    .map(|&(code, lbl)| {
        let s = if cur_mt == code { " selected" } else { "" };
        format!(r#"<option value="{code}"{s}>{lbl}</option>"#)
    })
    .collect::<Vec<_>>()
    .join("");

    let cur_sw = match sheet.social_wound {
        Some(SocialWoundKind::RhetoricalStrain) => "STRAIN",
        Some(SocialWoundKind::Doubt) => "DOUBT",
        Some(SocialWoundKind::CognitiveDissonance) => "DISSONANCE",
        Some(SocialWoundKind::EgoWound) => "EGO_WOUND",
        _ => "NONE",
    };
    let sw_opts: String = [
        ("NONE", "None (Unshaken Conviction)"),
        ("STRAIN", "Rhetorical Strain (+1 Δ_int on Social)"),
        ("DOUBT", "Doubt (+2 Δ_int, Hesitation)"),
        (
            "DISSONANCE",
            "Cognitive Dissonance (+4 Δ_int until resolved)",
        ),
        ("EGO_WOUND", "Ego Wound (+2 Δ_int, Rash Provocation)"),
    ]
    .iter()
    .map(|&(code, lbl)| {
        let s = if cur_sw == code { " selected" } else { "" };
        format!(r#"<option value="{code}"{s}>{lbl}</option>"#)
    })
    .collect::<Vec<_>>()
    .join("");

    let drift_thresh = psyche_drift_threshold(c.dis);
    let last_roll_badge = match sheet.last_hit_location_roll_d20 {
        Some(r) => format!(r#"<span class="pill pill-active">Last 1d20 Hit Roll: {r} ({})</span>"#, sel_spec.name),
        None => r#"<span class="pill pill-muted">Click "Roll 1d20 Hit Location" or any body region</span>"#.to_string(),
    };
    let surge_lbl = if sheet.conviction_surge_active {
        "Conviction Surge: ACTIVE (-6 Δ_int, +2 Pool)"
    } else {
        "Activate Conviction Surge (-6 Δ_int, +2 Pool)"
    };
    let human_svg = format_human_hit_location_svg(sheet);

    format!(
        r#"<section class="char-sheet-section" aria-label="Status Tracker for Damage, Fatigue, Psyche, and Human-Shaped Hit Locations">
            <div class="char-section-header">
                <div>
                    <h3 title="Anatomical Hit Locations, Damage/Bleed, Fatigue &amp; Psyche">4. Status Tracker</h3>
                    <span class="subtitle">Track localized broken bones, arterial/venous bleeding, systemic Wound Shock (`WSR`), Fatigue (`FT`), and Psyche/Morale.</span>
                </div>
                <div class="btn-row-compact">
                    {last_roll_badge}
                    <button type="button" class="btn btn-sm btn-accent" onclick="window.satyaCharSheetRollHitLocation()">Roll 1d20 Hit Location</button>
                    <button type="button" class="btn btn-sm btn-secondary" onclick="window.satyaCharSheetUpdateStatus('sync_to_roller', '')">Sync Δ_int ({delta_int:+}) to 2d20 Roller</button>
                </div>
            </div>

            <div class="char-hit-location-layout">
                <div class="char-human-diagram-card">
                    {human_svg}
                    <div class="btn-row-compact">
                        <button type="button" class="btn btn-sm btn-secondary" onclick="window.satyaCharSheetUpdateZone(0, 'stabilize_bleeds', '')">Stabilize All Bleeds</button>
                        <button type="button" class="btn btn-sm btn-secondary" onclick="window.satyaCharSheetUpdateZone(0, 'clear_all', '')">Heal / Reset All Zones</button>
                    </div>
                </div>

                <div class="char-status-right-col">
                    <div class="char-table-card">
                        <div class="char-table-title">
                            <span>Selected Hit Zone Inspector: <strong>[d20 {d20_min}–{d20_max}] {sel_name}</strong></span>
                            <span class="pill pill-iggy">Called Shot +{cs_ext} Δ_ext · Cap Slashes: -{wound_t}T / -{wound_c}C / -{wound_f}F</span>
                        </div>
                        <div class="stat-note">{sel_notes}</div>
                        <div class="char-zone-controls-grid">
                            <label class="inline-label" for="char-zone-wound-select">
                                <span>Structural Wound Tier</span>
                                <select
                                    id="char-zone-wound-select"
                                    onchange="window.satyaCharSheetUpdateZone({sel_idx}, 'wound_tier', this.value)"
                                >
                                    {wound_tier_opts}
                                </select>
                            </label>
                            <label class="inline-label" for="char-zone-bone-select">
                                <span>Bone / Joint Status</span>
                                <select
                                    id="char-zone-bone-select"
                                    onchange="window.satyaCharSheetUpdateZone({sel_idx}, 'bone_status', this.value)"
                                >
                                    {bone_opts}
                                </select>
                            </label>
                            <label class="inline-label" for="char-zone-bleed-select">
                                <span>Bleeding / Hemorrhage</span>
                                <select
                                    id="char-zone-bleed-select"
                                    onchange="window.satyaCharSheetUpdateZone({sel_idx}, 'bleed_grade', this.value)"
                                >
                                    {bleed_opts}
                                </select>
                            </label>
                        </div>
                    </div>

                    <div class="char-trackers-three-grid">
                        <!-- Damage, Shock & Blood Volume Tracker -->
                        <div class="char-tracker-box">
                            <div class="char-table-title">
                                <span>Damage, Shock &amp; Blood Track</span>
                                <span class="pill pill-danger">Shock {shock_idx}/5 · WSR {wsr}</span>
                            </div>
                            <div class="stat-note">
                                <strong>Blood Volume:</strong> {rem_blood:.2} / 10.0 Units ({lost_blood:.2} U lost)<br/>
                                <strong>State:</strong> {hypovolemic_lbl}
                            </div>
                            <div class="btn-row-compact">
                                <button type="button" class="btn btn-sm btn-secondary" onclick="window.satyaCharSheetUpdateStatus('wound_shock_delta', '-1')">Shock -1</button>
                                <button type="button" class="btn btn-sm btn-accent" onclick="window.satyaCharSheetUpdateStatus('wound_shock_delta', '1')">Shock +1</button>
                                <button type="button" class="btn btn-sm btn-secondary" onclick="window.satyaCharSheetUpdateStatus('blood_loss_delta', '-0.5')">Blood +0.5U</button>
                                <button type="button" class="btn btn-sm btn-accent" onclick="window.satyaCharSheetUpdateStatus('blood_loss_delta', '0.5')">Bleed -0.5U</button>
                                <button type="button" class="btn btn-sm btn-secondary" onclick="window.satyaCharSheetUpdateStatus('advance_bleed_ticks', '20')">+20t Bleed</button>
                            </div>
                        </div>

                        <!-- Fatigue & Exertion Tracker -->
                        <div class="char-tracker-box">
                            <div class="char-table-title">
                                <span>Fatigue &amp; Exertion Tracker</span>
                                <span class="pill pill-active">Level {fat_lvl}/5 ({fat_name})</span>
                            </div>
                            <div class="stat-note">
                                <strong>Exertion Marks:</strong> {ex_marks} / {ft} FT (Rolls over into +1 Fatigue)<br/>
                                <strong>Penalties:</strong> Δ_int {fat_dint:+}, δ_under {fat_dunder}, Pool {fat_pool:+}, Time {fat_time:.2}x · <em>Rec: {fat_rec}</em>
                            </div>
                            <div class="btn-row-compact">
                                <button type="button" class="btn btn-sm btn-secondary" onclick="window.satyaCharSheetUpdateStatus('fatigue_delta', '-1')">Fatigue -1</button>
                                <button type="button" class="btn btn-sm btn-accent" onclick="window.satyaCharSheetUpdateStatus('fatigue_delta', '1')">Fatigue +1</button>
                                <button type="button" class="btn btn-sm btn-accent" onclick="window.satyaCharSheetUpdateStatus('exertion_delta', '1')">+1 Exertion</button>
                                <button type="button" class="btn btn-sm btn-accent" onclick="window.satyaCharSheetUpdateStatus('exertion_delta', '2')">+2 Exertion</button>
                                <button type="button" class="btn btn-sm btn-secondary" onclick="window.satyaCharSheetUpdateStatus('exertion_delta', '-100')">Clear Marks</button>
                            </div>
                        </div>

                        <!-- Psyche, Morale & Composure Tracker -->
                        <div class="char-tracker-box">
                            <div class="char-table-title">
                                <span>Psyche, Morale &amp; Composure</span>
                                <span class="pill pill-iggy">CMP Lost {cmp_lost}/{cmp_max} · Drift {drift}/{drift_thresh}</span>
                            </div>
                            <div class="char-zone-controls-grid">
                                <label class="inline-label" for="char-morale-select">
                                    <span>Combat Morale</span>
                                    <select id="char-morale-select" onchange="window.satyaCharSheetUpdateStatus('morale_state', this.value)">{morale_opts}</select>
                                </label>
                                <label class="inline-label" for="char-mental-trauma-select">
                                    <span>Acute Mental Trauma</span>
                                    <select id="char-mental-trauma-select" onchange="window.satyaCharSheetUpdateStatus('mental_trauma', this.value)">{mt_opts}</select>
                                </label>
                                <label class="inline-label" for="char-social-wound-select">
                                    <span>Social / Dissonance Wound</span>
                                    <select id="char-social-wound-select" onchange="window.satyaCharSheetUpdateStatus('social_wound', this.value)">{sw_opts}</select>
                                </label>
                            </div>
                            <div class="btn-row-compact">
                                <button type="button" class="btn btn-sm btn-secondary" onclick="window.satyaCharSheetUpdateStatus('composure_delta', '-1')">CMP -1</button>
                                <button type="button" class="btn btn-sm btn-accent" onclick="window.satyaCharSheetUpdateStatus('composure_delta', '1')">CMP +1</button>
                                <button type="button" class="btn btn-sm btn-secondary" onclick="window.satyaCharSheetUpdateStatus('drift_delta', '-1')">Drift -1</button>
                                <button type="button" class="btn btn-sm btn-accent" onclick="window.satyaCharSheetUpdateStatus('drift_delta', '1')">Drift +1</button>
                                <button type="button" class="btn btn-sm btn-accent" onclick="window.satyaCharSheetUpdateStatus('toggle_surge', '')">{surge_lbl}</button>
                            </div>
                        </div>
                    </div>

                    <div class="roster-bar">
                        <span class="roster-title">Physiological &amp; Sensory Status Conditions (Click to Toggle Active Conditions):</span>
                        <div class="preset-users">{cond_buttons}</div>
                    </div>
                </div>
            </div>

            <div class="char-table-card">
                <div class="char-table-title">
                    <span>13-Zone Master Anatomical Hit Location Status Table (Click Any Row to Select &amp; Edit)</span>
                    <span class="pill pill-muted">1d20 Location · Wound Tier · Bone Integrity · Hemorrhage Grade</span>
                </div>
                <div class="char-table-scroll">
                    <table id="char-sheet-hit-locations-table" class="char-data-table">
                        <thead>
                            <tr>
                                <th>1d20 Roll</th>
                                <th>Anatomical Sub-Zone</th>
                                <th>Slashes</th>
                                <th>Structural Wound</th>
                                <th>Bones / Joints</th>
                                <th>Bleeding / Hemorrhage</th>
                            </tr>
                        </thead>
                        <tbody>{hit_table_rows}</tbody>
                    </table>
                </div>
            </div>
        </section>"#,
        d20_min = sel_spec.d20_min,
        d20_max = sel_spec.d20_max,
        sel_name = escape_html(sel_spec.name),
        cs_ext = sel_spec.called_shot_delta_ext,
        wound_t = wound_caps.tempo,
        wound_c = wound_caps.control,
        wound_f = wound_caps.force,
        sel_notes = escape_html(sel_spec.mechanical_notes),
        wsr = derived.wsr_wound_shock_resistance,
        rem_blood = blood_eval.remaining_blood_units,
        lost_blood = blood_eval.total_units_lost,
        fat_name = fat_spec.condition_name,
        ex_marks = sheet.exertion_marks,
        ft = derived.ft_fatigue_threshold,
        fat_dint = fat_spec.delta_int,
        fat_dunder = fat_spec.delta_under,
        fat_pool = fat_spec.tactical_pool_drain,
        fat_time = fat_spec.action_time_factor,
        fat_rec = fat_spec.recovery_requirement,
        cmp_lost = sheet.social_composure_lost,
        cmp_max = derived.cmp_social_composure_threshold,
        drift = sheet.psyche_drift_marks,
    )
}

/// Formats the Writable Operator Field Notes & Lifepath / Economy Dossier section.
#[must_use]
pub fn format_character_notes_and_lifepath_html(sheet: &CharacterSheetState) -> String {
    let sqm_spec = lookup_campaign_sqm(sheet.campaign_sqm);
    let heat_spec = evaluate_operational_heat(sheet.operational_heat);
    let access_pills: String = sheet
        .granted_access_tags
        .iter()
        .map(|tag| {
            format!(
                r#"<span class="pill pill-active">[Access: {}]</span>"#,
                escape_html(tag.label())
            )
        })
        .collect::<Vec<_>>()
        .join(" ");

    let history_items: String = sheet
        .stage_history
        .iter()
        .map(|entry| format!("<li>{}</li>", escape_html(entry)))
        .collect::<Vec<_>>()
        .join("");

    format!(
        r#"<section class="char-sheet-section" aria-label="Writable Operator Field Notes, Lifepath &amp; Economy Dossier">
            <div class="char-section-header">
                <div>
                    <h3>5. Operator Field Notes, Lifepath History &amp; Campaign Economy</h3>
                    <span class="subtitle">Writable scratchpad for session objectives, inventory annotations, NPCs, and tactical log entries.</span>
                </div>
                <div class="btn-row-compact">
                    <button type="button" class="btn btn-sm btn-secondary" onclick="window.satyaCharSheetSaveNotes('stamp_status')">+ Stamp Current Status</button>
                    <button type="button" class="btn btn-sm btn-secondary" onclick="window.satyaCharSheetSaveNotes('clear')">Clear Notes</button>
                </div>
            </div>

            <div class="char-notes-wrap">
                <label class="roster-title" for="char-sheet-notes-textarea">Operator Field Notes &amp; Mission Log (Editable):</label>
                <textarea
                    id="char-sheet-notes-textarea"
                    class="gm-doc-textarea char-notes-textarea"
                    rows="5"
                    placeholder="Write character notes, mission objectives, ammo/consumable tracking, contacts, or injury log..."
                    oninput="window.satyaCharSheetSaveNotes('input')"
                >{notes}</textarea>
            </div>

            <div id="char-sheet-lifepath-summary" class="roster-bar">
                <span class="roster-title">Lifepath, [Access] Gates &amp; Economy:</span>
                <div class="roster-chips">
                    <span class="pill pill-iggy">Origin: {origin}</span>
                    <span class="pill pill-iggy">SQM {sqm}: ${usd}/mo ({tier_name})</span>
                    <span class="pill pill-iggy">Heat {heat}/10 (+{h_delta} Δ_ext)</span>
                    {access_pills}
                </div>
                <ul class="char-lifepath-list">{history_items}</ul>
            </div>
        </section>"#,
        notes = escape_html(&sheet.notes),
        origin = escape_html(&sheet.origin_label),
        sqm = sqm_spec.sqm_tier,
        usd = sqm_spec.monthly_liquid_baseline_usd,
        tier_name = escape_html(sqm_spec.tier_name),
        heat = heat_spec.heat_score,
        h_delta = heat_spec.public_check_delta_ext,
    )
}
