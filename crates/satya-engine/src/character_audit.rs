//! Deterministic Anti-Cheat Auditor & Canonical Schema Record Conversions for Character Sheets.
//!
//! Connects [`satya_schema::CharacterSheetRecord`] with [`crate::CharacterCharacteristics`],
//! [`crate::CharacterAptitudes`], [`crate::LifepathBuildConfig`], the 5-Factor SP Ledger
//! ([`crate::compute_skill_allocation_sp_ledger`]), and the 4-Gate Skill DAG Validator
//! ([`crate::validate_skill_advancement`]).

use std::collections::BTreeMap;

use satya_schema::{
    CharacterAptitudesRecord, CharacterCharacteristicsRecord, CharacterSheetRecord,
};
use serde::{Deserialize, Serialize};

use crate::characteristics::{
    AptitudeCode, CharacterAptitudes, CharacterCharacteristics, CharacteristicCode,
};
use crate::economy_gear::compute_skill_allocation_sp_ledger;
use crate::lifepath_economy::{LifepathBuildConfig, compute_lifepath_baseline_from_config};
use crate::skills_trees::{SkillAccessTier, lookup_skill_node, validate_skill_advancement};

/// Convert [`CharacterCharacteristics`] into the serializable schema [`CharacterCharacteristicsRecord`].
#[must_use]
pub fn characteristics_to_record(c: &CharacterCharacteristics) -> CharacterCharacteristicsRecord {
    CharacterCharacteristicsRecord {
        spd: c.spd,
        for_stat: c.for_stat,
        dex: c.dex,
        kin: c.kin,
        hlt: c.hlt,
        fit: c.fit,
        hgt_m: c.hgt_m,
        mas_kg: c.mas_kg,
        mem: c.mem,
        ana: c.ana,
        ins: c.ins,
        cre: c.cre,
        dis: c.dis,
        foc: c.foc,
        com: c.com,
        pre: c.pre,
        elo: c.elo,
        sig: c.sig,
        hea: c.hea,
        tac: c.tac,
        olf: c.olf,
        tas: c.tas,
        itu: c.itu,
        ego: c.ego,
        slf: c.slf,
        opn: c.opn,
        blg: c.blg,
        ext: c.ext,
        cnf: c.cnf,
        agg: c.agg,
    }
}

/// Convert [`CharacterCharacteristicsRecord`] into [`CharacterCharacteristics`] (clamping out-of-range values).
#[must_use]
pub fn characteristics_from_record(r: &CharacterCharacteristicsRecord) -> CharacterCharacteristics {
    CharacterCharacteristics {
        spd: r.spd.clamp(1, 20),
        for_stat: r.for_stat.clamp(1, 20),
        dex: r.dex.clamp(1, 20),
        kin: r.kin.clamp(1, 20),
        hlt: r.hlt.clamp(1, 20),
        fit: r.fit.clamp(1, 20),
        hgt_m: r.hgt_m.clamp(0.5, 3.0),
        mas_kg: r.mas_kg.clamp(20.0, 300.0),
        mem: r.mem.clamp(1, 20),
        ana: r.ana.clamp(1, 20),
        ins: r.ins.clamp(1, 20),
        cre: r.cre.clamp(1, 20),
        dis: r.dis.clamp(1, 20),
        foc: r.foc.clamp(1, 20),
        com: r.com.clamp(1, 20),
        pre: r.pre.clamp(1, 20),
        elo: r.elo.clamp(1, 20),
        sig: r.sig.clamp(1, 20),
        hea: r.hea.clamp(1, 20),
        tac: r.tac.clamp(1, 20),
        olf: r.olf.clamp(1, 20),
        tas: r.tas.clamp(1, 20),
        itu: r.itu.clamp(1, 20),
        ego: r.ego.clamp(1, 20),
        slf: r.slf.clamp(1, 20),
        opn: r.opn.clamp(1, 20),
        blg: r.blg.clamp(1, 20),
        ext: r.ext.clamp(1, 20),
        cnf: r.cnf.clamp(1, 20),
        agg: r.agg.clamp(1, 20),
    }
}

/// Convert [`CharacterAptitudes`] into the serializable schema [`CharacterAptitudesRecord`].
#[must_use]
pub fn aptitudes_to_record(a: &CharacterAptitudes) -> CharacterAptitudesRecord {
    CharacterAptitudesRecord {
        apt_qnt: a.apt_qnt,
        apt_lng: a.apt_lng,
        apt_bio: a.apt_bio,
        apt_mec: a.apt_mec,
        apt_cmp: a.apt_cmp,
        apt_kin: a.apt_kin,
        apt_fin: a.apt_fin,
        apt_art: a.apt_aud,
        apt_soc: a.apt_soc,
    }
}

/// Convert [`CharacterAptitudesRecord`] into [`CharacterAptitudes`] (clamping to `-2..=3`).
#[must_use]
pub fn aptitudes_from_record(r: &CharacterAptitudesRecord) -> CharacterAptitudes {
    CharacterAptitudes {
        apt_qnt: r.apt_qnt.clamp(-2, 3),
        apt_lng: r.apt_lng.clamp(-2, 3),
        apt_bio: r.apt_bio.clamp(-2, 3),
        apt_mec: r.apt_mec.clamp(-2, 3),
        apt_cmp: r.apt_cmp.clamp(-2, 3),
        apt_kin: r.apt_kin.clamp(-2, 3),
        apt_fin: r.apt_fin.clamp(-2, 3),
        apt_aud: r.apt_art.clamp(-2, 3),
        apt_soc: r.apt_soc.clamp(-2, 3),
    }
}

/// Deterministic anti-cheat and ruleset legality report produced by [`verify_character_sheet_legality`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterAuditReport {
    /// Whether the character passes all characteristic, aptitude, DAG prerequisite, and SP budget checks.
    pub is_legal: bool,
    /// Whether all 28 scaled characteristics are in `1..=20` and height/mass/aptitudes are in range.
    pub characteristics_valid: bool,
    /// Whether all trained skills satisfy `[Access]`, `Min C`, parent prerequisite, and Foundation Ceiling gates.
    pub dag_prereqs_valid: bool,
    /// Whether post-school skill advancements fit within the lifepath Formal + Experience SP budgets.
    pub sp_budget_valid: bool,
    /// Formal SP spent (when `lifepath_config_json` is present).
    pub formal_sp_spent: usize,
    /// Formal SP budget (when `lifepath_config_json` is present).
    pub formal_sp_budget: usize,
    /// Experience SP spent (when `lifepath_config_json` is present).
    pub experience_sp_spent: usize,
    /// Experience SP budget (when `lifepath_config_json` is present).
    pub experience_sp_budget: usize,
    /// Human-readable audit violations (empty when `is_legal == true`).
    pub violations: Vec<String>,
}

/// Deterministically audits a [`CharacterSheetRecord`] against all Satya character creation and advancement rules:
/// 1. All 28 scaled Characteristics in `1..=20`, `hgt_m` in `0.5..=3.0`, `mas_kg` in `20.0..=300.0`, and all 9 Domain Aptitudes in `-2..=+3`.
/// 2. All skill ranks in `0..=10` (`TAL` innate talents $\ge 1$) and valid canonical skill slugs.
/// 3. All 4 canonical Skill Tree DAG gates (`[Access]`, `Min C`, Parent Prerequisite, and Foundation Ceiling $N_{\text{child}} \le N_{\text{parent}} + 2$).
/// 4. If `lifepath_config_json` is provided, recomputes the Stage 1–5 lifepath baseline and 5-Factor SP Ledger and verifies
///    `formal_sp_spent <= formal_sp_budget` and `experience_sp_spent <= experience_sp_budget`.
#[must_use]
pub fn verify_character_sheet_legality(record: &CharacterSheetRecord) -> CharacterAuditReport {
    let mut violations = Vec::new();
    let mut characteristics_valid = true;
    let mut dag_prereqs_valid = true;
    let mut sp_budget_valid = true;

    let c_rec = &record.characteristics;
    let raw_scaled = [
        (CharacteristicCode::SPD, c_rec.spd),
        (CharacteristicCode::FOR, c_rec.for_stat),
        (CharacteristicCode::DEX, c_rec.dex),
        (CharacteristicCode::KIN, c_rec.kin),
        (CharacteristicCode::HLT, c_rec.hlt),
        (CharacteristicCode::FIT, c_rec.fit),
        (CharacteristicCode::MEM, c_rec.mem),
        (CharacteristicCode::ANA, c_rec.ana),
        (CharacteristicCode::INS, c_rec.ins),
        (CharacteristicCode::CRE, c_rec.cre),
        (CharacteristicCode::DIS, c_rec.dis),
        (CharacteristicCode::FOC, c_rec.foc),
        (CharacteristicCode::COM, c_rec.com),
        (CharacteristicCode::PRE, c_rec.pre),
        (CharacteristicCode::ELO, c_rec.elo),
        (CharacteristicCode::SIG, c_rec.sig),
        (CharacteristicCode::HEA, c_rec.hea),
        (CharacteristicCode::TAC, c_rec.tac),
        (CharacteristicCode::OLF, c_rec.olf),
        (CharacteristicCode::TAS, c_rec.tas),
        (CharacteristicCode::ITU, c_rec.itu),
        (CharacteristicCode::EGO, c_rec.ego),
        (CharacteristicCode::SLF, c_rec.slf),
        (CharacteristicCode::OPN, c_rec.opn),
        (CharacteristicCode::BLG, c_rec.blg),
        (CharacteristicCode::EXT, c_rec.ext),
        (CharacteristicCode::CNF, c_rec.cnf),
        (CharacteristicCode::AGG, c_rec.agg),
    ];
    for (code, val) in raw_scaled {
        if !(1..=20).contains(&val) {
            characteristics_valid = false;
            violations.push(format!(
                "Characteristic {} = {} is outside legal scale 1..=20.",
                code.as_str(),
                val
            ));
        }
    }
    if !(0.5..=3.0).contains(&c_rec.hgt_m) {
        characteristics_valid = false;
        violations.push(format!(
            "Height HGT = {:.2}m is outside legal range 0.50..=3.00m.",
            c_rec.hgt_m
        ));
    }
    if !(20.0..=300.0).contains(&c_rec.mas_kg) {
        characteristics_valid = false;
        violations.push(format!(
            "Mass MAS = {:.1}kg is outside legal range 20.0..=300.0kg.",
            c_rec.mas_kg
        ));
    }

    let a_rec = &record.aptitudes;
    let raw_apts = [
        (AptitudeCode::AptQnt, a_rec.apt_qnt),
        (AptitudeCode::AptLng, a_rec.apt_lng),
        (AptitudeCode::AptBio, a_rec.apt_bio),
        (AptitudeCode::AptMec, a_rec.apt_mec),
        (AptitudeCode::AptCmp, a_rec.apt_cmp),
        (AptitudeCode::AptKin, a_rec.apt_kin),
        (AptitudeCode::AptFin, a_rec.apt_fin),
        (AptitudeCode::AptAud, a_rec.apt_art),
        (AptitudeCode::AptSoc, a_rec.apt_soc),
    ];
    for (code, val) in raw_apts {
        if !(-2..=3).contains(&val) {
            characteristics_valid = false;
            violations.push(format!(
                "Domain Aptitude {} = {:+} is outside legal range -2..=+3.",
                code.as_str(),
                val
            ));
        }
    }

    let chars = characteristics_from_record(c_rec);
    let aptitudes = aptitudes_from_record(a_rec);

    let mut skills_usize = BTreeMap::new();
    for (sk_id, &rank) in &record.skills {
        if !(0..=10).contains(&rank) {
            dag_prereqs_valid = false;
            violations.push(format!(
                "Skill '{}' rank N={} is outside legal range 0..=10.",
                sk_id, rank
            ));
        }
        if lookup_skill_node(sk_id).is_none() {
            dag_prereqs_valid = false;
            violations.push(format!(
                "Unknown skill '{}' not in canonical 8-tree registry.",
                sk_id
            ));
        } else {
            skills_usize.insert(sk_id.clone(), rank.clamp(0, 10) as usize);
        }
    }

    let mut granted_access: Vec<SkillAccessTier> = record
        .granted_access_tags
        .iter()
        .filter_map(|s| SkillAccessTier::from_code_str(s))
        .collect();
    if !granted_access.contains(&SkillAccessTier::Universal) {
        granted_access.push(SkillAccessTier::Universal);
    }

    let mut formal_sp_spent = 0usize;
    let mut formal_sp_budget = 0usize;
    let mut experience_sp_spent = 0usize;
    let mut experience_sp_budget = 0usize;
    let mut baseline_skills_opt = None;

    if let Some(cfg_json) = record
        .lifepath_config_json
        .as_deref()
        .filter(|s| !s.trim().is_empty())
    {
        match serde_json::from_str::<LifepathBuildConfig>(cfg_json) {
            Ok(cfg) => {
                let baseline = compute_lifepath_baseline_from_config(&cfg, &chars, &aptitudes);
                for &tag in &baseline.granted_access_tags {
                    if !granted_access.contains(&tag) {
                        granted_access.push(tag);
                    }
                }
                let ledger = compute_skill_allocation_sp_ledger(
                    &baseline,
                    &cfg,
                    &chars,
                    &aptitudes,
                    &skills_usize,
                );
                formal_sp_spent = ledger.formal_sp_spent;
                formal_sp_budget = ledger.formal_sp_budget;
                experience_sp_spent = ledger.experience_sp_spent;
                experience_sp_budget = ledger.experience_sp_budget;

                if ledger.formal_sp_spent > ledger.formal_sp_budget {
                    sp_budget_valid = false;
                    violations.push(format!(
                        "Formal SP over budget: spent {} SP > budget {} SP.",
                        ledger.formal_sp_spent, ledger.formal_sp_budget
                    ));
                }
                if ledger.experience_sp_spent > ledger.experience_sp_budget {
                    sp_budget_valid = false;
                    violations.push(format!(
                        "Experience SP over budget: spent {} SP > budget {} SP.",
                        ledger.experience_sp_spent, ledger.experience_sp_budget
                    ));
                }
                baseline_skills_opt =
                    Some((baseline.baseline_skills, baseline.academic_tree_frozen));
            }
            Err(err) => {
                sp_budget_valid = false;
                violations.push(format!("Invalid lifepath_config_json: {err}"));
            }
        }
    }

    // Validate 4-gate DAG prerequisites:
    // - When lifepath_config_json is present, we have the exact Stage 1–4 school/archetype baseline floor
    //   and can audit every post-school advancement against all 4 gates (including Academic Freeze and Foundation Ceiling).
    // - When lifepath_config_json is omitted (e.g. a hand-crafted or preset fixture without Stage 1–4 history),
    //   we validate Min C thresholds, Talent floors, and non-Academic Access tags (since Academic skills <= N=3 may have been acquired in school before any dropout).
    if let Some((baseline_skills, academic_frozen)) = &baseline_skills_opt {
        for (sk_id, &cur_n) in &skills_usize {
            let Some(spec) = lookup_skill_node(sk_id) else {
                continue;
            };
            let floor_n = baseline_skills.get(sk_id).copied().unwrap_or_else(|| {
                usize::from(spec.skill_type == crate::skills_trees::SkillType::Tal)
            });

            if cur_n > floor_n {
                let mut skills_before_target = skills_usize.clone();
                skills_before_target.insert(sk_id.clone(), cur_n - 1);
                let val = validate_skill_advancement(
                    sk_id,
                    cur_n,
                    &skills_before_target,
                    &chars,
                    &aptitudes,
                    &granted_access,
                    *academic_frozen,
                );
                if !val.is_valid {
                    dag_prereqs_valid = false;
                    for f in val.failures {
                        violations.push(format!("[{}] {}", f.code, f.detail));
                    }
                }
            }
        }
    } else {
        let mut custom_access = granted_access.clone();
        if !custom_access.contains(&SkillAccessTier::Academic) {
            custom_access.push(SkillAccessTier::Academic);
        }
        for (sk_id, &cur_n) in &skills_usize {
            let Some(spec) = lookup_skill_node(sk_id) else {
                continue;
            };
            let floor_n = usize::from(spec.skill_type == crate::skills_trees::SkillType::Tal);
            if cur_n > floor_n {
                let mut skills_before_target = skills_usize.clone();
                skills_before_target.insert(sk_id.clone(), cur_n - 1);
                let val = validate_skill_advancement(
                    sk_id,
                    cur_n,
                    &skills_before_target,
                    &chars,
                    &aptitudes,
                    &custom_access,
                    false,
                );
                for f in val.failures {
                    if f.code == "MISSING_ACCESS_GATE"
                        || f.code == "MIN_CHARACTERISTIC_SHORTFALL"
                        || (f.code == "PARENT_PREREQ_SHORTFALL" && spec.tier >= 3)
                    {
                        dag_prereqs_valid = false;
                        violations.push(format!("[{}] {}", f.code, f.detail));
                    }
                }
            }
        }
    }

    let is_legal =
        characteristics_valid && dag_prereqs_valid && sp_budget_valid && violations.is_empty();

    CharacterAuditReport {
        is_legal,
        characteristics_valid,
        dag_prereqs_valid,
        sp_budget_valid,
        formal_sp_spent,
        formal_sp_budget,
        experience_sp_spent,
        experience_sp_budget,
        violations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::walkthroughs::build_mateo_vance_lifepath;

    #[test]
    fn test_verify_character_sheet_legality_detects_tampering_and_accepts_legal_builds() {
        let mateo = build_mateo_vance_lifepath();
        let mut skills = BTreeMap::new();
        for (k, v) in &mateo.skills {
            skills.insert(k.clone(), *v as i32);
        }
        let mut rec = CharacterSheetRecord {
            character_id: "char_mateo".to_string(),
            owner_user_id: "u-test".to_string(),
            character_name: mateo.name.clone(),
            age_years: mateo.age_years as i32,
            origin_label: mateo.origin.label.to_string(),
            campaign_sqm: i32::from(mateo.campaign_sqm),
            operational_heat: i32::from(mateo.operational_heat),
            granted_access_tags: mateo
                .granted_access_tags
                .iter()
                .map(|t| t.as_str().to_string())
                .collect(),
            stage_history: mateo.stage_history.clone(),
            characteristics: characteristics_to_record(&mateo.characteristics),
            aptitudes: aptitudes_to_record(&mateo.aptitudes),
            skills,
            ..CharacterSheetRecord::default()
        };

        let report_ok = verify_character_sheet_legality(&rec);
        assert!(
            report_ok.is_legal,
            "Canonical Mateo Vance build should be legal: {:?}",
            report_ok.violations
        );

        // Tamper with characteristic outside 1..=20 and add an un-gated Tier 5 surgery skill at N=5
        rec.characteristics.for_stat = 25;
        rec.skills
            .insert("trauma_orthopedic_general_surgery".to_string(), 5);
        let report_bad = verify_character_sheet_legality(&rec);
        assert!(!report_bad.is_legal);
        assert!(!report_bad.characteristics_valid);
        assert!(!report_bad.dag_prereqs_valid);
        assert!(report_bad.violations.len() >= 2);
    }
}
