//! Pure HTML formatters for Section 3 of the Character Sheet (`#notebook-pane-character`):
//! Ranged Firearms (with live physics-derived Ballistics), Melee Weapons, Armor Profiles,
//! and Field Gear / Ergonomic Encumbrance tables.
//!
//! Extracted from [`crate::wasm_client::character_sheet_views`] so core characteristic/skill
//! tables and equipment loadout tables each fit comfortably under `25 KB`.

use crate::wasm_client::character_sheet::{
    CANONICAL_FIELD_GEAR_PRESETS, CANONICAL_FIREARM_PRESETS, CharacterSheetState,
    lookup_firearm_preset,
};
use crate::wasm_client::dom::{escape_html, escape_js_single_quoted_attr};
use satya_engine::{
    ARMOR_CATALOG, MELEE_WEAPON_CATALOG, RANGED_FIRE_MODES, evaluate_granular_firearm_ballistics,
};

/// Formats the Equipment Loadout section: Ranged Firearms, Melee Weapons, Armor Profiles, and Field Gear / Encumbrance.
#[must_use]
pub fn format_equipment_weapons_and_armor_html(sheet: &CharacterSheetState) -> String {
    let c = &sheet.characteristics;
    let derived = c.compute_derived(0);
    let enc = sheet.compute_encumbrance();
    let dist_m = sheet.ballistic_range_m;

    let ranged_rows: String = sheet
        .equipped_ranged_ids
        .iter()
        .filter_map(|r_id| {
            let spec = lookup_firearm_preset(r_id)?;
            let bal = evaluate_granular_firearm_ballistics(spec, dist_m);
            let pid_js = escape_js_single_quoted_attr(r_id);
            Some(format!(
                r#"<tr>
                    <td>
                        <strong>{name}</strong>
                        <div class="stat-note">{ammo} · {oal:.0}cm OAL ({bbl:.1}cm bbl) · ${cost}</div>
                    </td>
                    <td class="mono">{cap} rds</td>
                    <td class="mono"><strong>{e_imp:.0} J</strong> <span class="stat-note">(Muz {e_muz:.0}J)</span></td>
                    <td class="mono"><strong>{acc:+}</strong> <span class="stat-note">(Yaw {yaw})</span></td>
                    <td class="mono"><strong>R = {rec}</strong></td>
                    <td class="mono"><strong>Imp {imp}</strong> / <strong>Pen {pen}</strong></td>
                    <td class="mono">Rel {rel:+} <span class="stat-note">({jam}t/{strip}t)</span></td>
                    <td class="mono">{wt:.2} kg</td>
                    <td>
                        <button type="button" class="btn btn-sm btn-secondary" onclick="window.satyaCharSheetModifyEquipment('remove_ranged', '{pid_js}')">Remove</button>
                    </td>
                </tr>"#,
                name = escape_html(spec.name),
                ammo = escape_html(spec.ammo.name),
                oal = spec.overall_length_cm,
                bbl = spec.barrel_length_cm,
                cost = spec.cost_usd,
                cap = spec.capacity_rounds,
                e_imp = bal.impact_energy_joules,
                e_muz = bal.muzzle_energy_joules,
                acc = bal.accuracy_acc,
                yaw = bal.wobble_yaw_factor,
                rec = bal.base_recoil_r,
                imp = bal.raw_ballistic_impact,
                pen = bal.base_armor_pen,
                rel = bal.reliability_rel,
                jam = bal.clear_jam_minor_ticks,
                strip = bal.strip_time_ticks,
                wt = spec.loaded_weight_kg,
            ))
        })
        .collect::<Vec<_>>()
        .join("");

    let ranged_options: String = CANONICAL_FIREARM_PRESETS
        .iter()
        .map(|(pid, spec)| {
            format!(
                r#"<option value="{pid}">{} ({})</option>"#,
                escape_html(spec.name),
                escape_html(spec.ammo.name)
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let fire_modes_chips: String = RANGED_FIRE_MODES
        .iter()
        .map(|fm| {
            format!(
                r#"<span class="pill pill-muted" title="Rounds: {}-{}">{} (TC {}-{}, ΔR +{}-{})</span>"#,
                fm.min_rounds_expended,
                fm.max_rounds_expended,
                escape_html(fm.name),
                fm.min_base_tc,
                fm.max_base_tc,
                fm.min_mode_recoil_adder,
                fm.max_mode_recoil_adder,
            )
        })
        .collect::<Vec<_>>()
        .join(" ");

    let melee_rows: String = sheet
        .equipped_melee
        .iter()
        .map(|&mw_id| {
            let spec = mw_id.spec();
            let idx = mw_id as usize;
            let total_reach = derived.height_reach_edge + (c.kin - 10).div_euclid(4) + spec.max_reach;
            let base_impact = derived.pow_striking_power * spec.pow_multiplier
                + f64::from(spec.flat_trauma_or_bleed_bonus);
            let keep_out = if spec.has_keep_out_property {
                "Keep-Out (+4 Δ_ext)"
            } else {
                "Standard"
            };
            format!(
                r#"<tr>
                    <td><strong>{name}</strong></td>
                    <td class="mono">Reach {min_r}–{max_r} <span class="stat-note">(Total {total_reach:+})</span></td>
                    <td class="mono">{tc} Ticks</td>
                    <td class="mono">{guard:+} Guard</td>
                    <td class="mono"><strong>{base_impact:.1}</strong> <span class="stat-note">({mult:.2}x POW + {flat})</span></td>
                    <td><span class="stat-note">{keep_out} · +{blk} Sweep</span></td>
                    <td>
                        <button type="button" class="btn btn-sm btn-secondary" onclick="window.satyaCharSheetModifyEquipment('remove_melee', '{idx}')">Remove</button>
                    </td>
                </tr>"#,
                name = escape_html(spec.name),
                min_r = spec.min_reach,
                max_r = spec.max_reach,
                tc = spec.base_tc,
                guard = spec.guard_bonus,
                mult = spec.pow_multiplier,
                flat = spec.flat_trauma_or_bleed_bonus,
                blk = spec.block_sweep_force_bonus,
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let melee_options: String = MELEE_WEAPON_CATALOG
        .iter()
        .enumerate()
        .map(|(idx, spec)| {
            format!(
                r#"<option value="{idx}">{} (Reach {}-{}, TC {})</option>"#,
                escape_html(spec.name),
                spec.min_reach,
                spec.max_reach,
                spec.base_tc
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let armor_rows: String = sheet
        .equipped_armor
        .iter()
        .map(|&ar_id| {
            let spec = ar_id.spec();
            let idx = ar_id as usize;
            let exo_note = if spec.exo_load_offset_kg > 0.0 {
                format!("Powered Exo (-{:.1} kg offset)", spec.exo_load_offset_kg)
            } else if spec.heat_retention_delta > 0 {
                format!("Heat Retention +{}", spec.heat_retention_delta)
            } else {
                "Passive Armor".to_string()
            };
            format!(
                r#"<tr>
                    <td>
                        <strong>{name}</strong>
                        <div class="stat-note">${cost} · {exo_note}</div>
                    </td>
                    <td>{zones}</td>
                    <td class="mono"><strong>PR {bal}</strong></td>
                    <td class="mono">Edged {edg} / Ice {ice} / Visor {vis}</td>
                    <td class="mono"><strong>BTD {btd}</strong></td>
                    <td class="mono">{wt:.1} kg</td>
                    <td>
                        <button type="button" class="btn btn-sm btn-secondary" onclick="window.satyaCharSheetModifyEquipment('remove_armor', '{idx}')">Remove</button>
                    </td>
                </tr>"#,
                name = escape_html(spec.name),
                cost = spec.cost_usd,
                zones = escape_html(spec.encumbered_zones),
                bal = spec.pr_ballistic,
                edg = spec.pr_edged,
                ice = spec.pr_icepick_flechette,
                vis = spec.pr_visor_weak_spot,
                btd = spec.blunt_trauma_dampening_btd,
                wt = spec.weight_kg,
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let armor_options: String = ARMOR_CATALOG
        .iter()
        .enumerate()
        .map(|(idx, spec)| {
            format!(
                r#"<option value="{idx}">{} (Ballistic PR {}, BTD {}, {:.1}kg)</option>"#,
                escape_html(spec.name),
                spec.pr_ballistic,
                spec.blunt_trauma_dampening_btd,
                spec.weight_kg
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let gear_rows: String = sheet
        .field_gear
        .iter()
        .map(|g| {
            let id_js = escape_js_single_quoted_attr(&g.id);
            format!(
                r#"<tr>
                    <td><strong>{name}</strong></td>
                    <td><span class="wiz-char-code">{cat}</span></td>
                    <td class="mono">{mod_val:+} Δ_tool</td>
                    <td class="mono">{wt:.1} kg <span class="stat-note">({erg:.1}x)</span></td>
                    <td><span class="stat-note">{notes}</span></td>
                    <td>
                        <button type="button" class="btn btn-sm btn-secondary" onclick="window.satyaCharSheetModifyEquipment('remove_gear', '{id_js}')">Remove</button>
                    </td>
                </tr>"#,
                name = escape_html(&g.name),
                cat = escape_html(&g.category),
                mod_val = g.tool_mod,
                wt = g.mass_kg,
                erg = g.carry_method.ergonomic_factor(),
                notes = escape_html(&g.notes),
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let gear_options: String = CANONICAL_FIELD_GEAR_PRESETS
        .iter()
        .map(|&(pid, name, cat, mass_kg, _, tool_mod, _)| {
            format!(
                r#"<option value="{pid}">[{cat}] {} ({mass_kg:.1}kg, Δ_tool {tool_mod:+})</option>"#,
                escape_html(name),
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let range_options: String = [
        (15.0, "15m (CQB / Indoor)"),
        (25.0, "25m (Standard Engagement)"),
        (50.0, "50m (Medium Urban)"),
        (150.0, "150m (Rifle Standoff)"),
        (400.0, "400m (Long-Range Overwatch)"),
    ]
    .iter()
    .map(|&(r, lbl)| {
        let sel = if (dist_m - r).abs() < 1.0 { " selected" } else { "" };
        format!(r#"<option value="{r}"{sel}>{lbl}</option>"#)
    })
    .collect::<Vec<_>>()
    .join("");

    format!(
        r#"<section class="char-sheet-section" aria-label="Equipment, Ranged Weapons, Melee Weapons, Armor &amp; Encumbrance">
            <div class="char-section-header">
                <div>
                    <h3>3. Operator Equipment Loadout: Weapons, Armor &amp; Field Gear</h3>
                    <span class="subtitle">Physics-derived firearm ballistics (`E_k`, `Acc`, `Recoil R`, `Pen`, `Rel`), Melee Reach/POW scaling, Armor `PR`/`BABT`, and Ergonomic Encumbrance.</span>
                </div>
                <span class="pill pill-active">
                    Load Tier {tier_num} ({tier_lbl}) · Raw {raw_kg:.1} kg / Eff {eff_kg:.1} kg (Comfort {base_kg:.1} kg · Max Drag {drag_kg:.0} kg)
                </span>
            </div>

            <div class="char-table-card">
                <div class="char-table-title">
                    <span>Ranged Weapons &amp; Granular Ballistics Table</span>
                    <div class="model-controls-row">
                        <label class="inline-label" for="char-ballistic-range-select">
                            <span>Target Range</span>
                            <select id="char-ballistic-range-select" onchange="window.satyaCharSheetModifyEquipment('set_range', this.value)">{range_options}</select>
                        </label>
                        <select id="char-add-ranged-select" aria-label="Add Ranged Firearm">{ranged_options}</select>
                        <button type="button" class="btn btn-sm btn-accent" onclick="window.satyaCharSheetModifyEquipment('add_ranged', '')">+ Equip Firearm</button>
                    </div>
                </div>
                <div class="char-table-scroll">
                    <table id="char-sheet-ranged-table" class="char-data-table">
                        <thead>
                            <tr>
                                <th>Firearm &amp; Cartridge</th>
                                <th>Mag</th>
                                <th>Energy @ {dist_m:.0}m</th>
                                <th>Acc / Yaw</th>
                                <th>Recoil (R)</th>
                                <th>Impact / Pen</th>
                                <th>Rel (Jam/Strip)</th>
                                <th>Mass</th>
                                <th>Action</th>
                            </tr>
                        </thead>
                        <tbody>{ranged_rows}</tbody>
                    </table>
                </div>
                <div class="roster-bar">
                    <span class="roster-title">Canonical Ranged Fire Modes (Braced when Force F &gt;= Recoil R):</span>
                    <div class="roster-chips">{fire_modes_chips}</div>
                </div>
            </div>

            <div class="char-tables-two-col">
                <div class="char-table-card">
                    <div class="char-table-title">
                        <span>Melee &amp; Unarmed Weapons</span>
                        <div class="btn-row-compact">
                            <select id="char-add-melee-select" aria-label="Add Melee Weapon">{melee_options}</select>
                            <button type="button" class="btn btn-sm btn-accent" onclick="window.satyaCharSheetModifyEquipment('add_melee', '')">+ Add Melee</button>
                        </div>
                    </div>
                    <div class="char-table-scroll">
                        <table id="char-sheet-melee-table" class="char-data-table">
                            <thead>
                                <tr>
                                    <th>Melee Weapon</th>
                                    <th>Reach</th>
                                    <th>Base TC</th>
                                    <th>Guard</th>
                                    <th>Strike Impact</th>
                                    <th>Properties</th>
                                    <th>Action</th>
                                </tr>
                            </thead>
                            <tbody>{melee_rows}</tbody>
                        </table>
                    </div>
                </div>

                <div class="char-table-card">
                    <div class="char-table-title">
                        <span>Equipped Armor &amp; Protection (PR vs. BABT)</span>
                        <div class="btn-row-compact">
                            <select id="char-add-armor-select" aria-label="Add Armor">{armor_options}</select>
                            <button type="button" class="btn btn-sm btn-accent" onclick="window.satyaCharSheetModifyEquipment('add_armor', '')">+ Equip Armor</button>
                        </div>
                    </div>
                    <div class="char-table-scroll">
                        <table id="char-sheet-armor-table" class="char-data-table">
                            <thead>
                                <tr>
                                    <th>Armor Profile</th>
                                    <th>Protected Zones</th>
                                    <th>Ballistic PR</th>
                                    <th>Edged / Ice / Visor</th>
                                    <th>Blunt BTD</th>
                                    <th>Mass</th>
                                    <th>Action</th>
                                </tr>
                            </thead>
                            <tbody>{armor_rows}</tbody>
                        </table>
                    </div>
                </div>
            </div>

            <div class="char-table-card">
                <div class="char-table-title">
                    <span>Field Equipment, Medical/Technical Kits &amp; Load Distribution</span>
                    <div class="btn-row-compact">
                        <select id="char-add-gear-select" aria-label="Add Field Gear">{gear_options}</select>
                        <button type="button" class="btn btn-sm btn-accent" onclick="window.satyaCharSheetModifyEquipment('add_gear', '')">+ Add Gear</button>
                    </div>
                </div>
                <div class="char-table-scroll">
                    <table id="char-sheet-gear-table" class="char-data-table">
                        <thead>
                            <tr>
                                <th>Equipment / Kit</th>
                                <th>Category</th>
                                <th>Tool Mod</th>
                                <th>Mass (Ergonomic)</th>
                                <th>Operational Notes</th>
                                <th>Action</th>
                            </tr>
                        </thead>
                        <tbody>{gear_rows}</tbody>
                    </table>
                </div>
            </div>
        </section>"#,
        tier_num = enc.tier.tier_number(),
        tier_lbl = match enc.tier.tier_number() {
            0 => "Unencumbered",
            1 => "Light / Patrol Load",
            2 => "Tactical / Fighting Load",
            3 => "Heavy / March Load",
            _ => "Overloaded / Staggering",
        },
        raw_kg = enc.raw_carried_mass_kg,
        eff_kg = enc.effective_load_kg,
        base_kg = enc.base_comfort_load_kg,
        drag_kg = enc.max_deadlift_drag_limit_kg,
    )
}
