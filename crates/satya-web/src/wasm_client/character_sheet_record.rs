//! Pure `CharacterSheetState` <-> [`satya_schema::CharacterSheetRecord`] conversion, anti-cheat
//! legality audit synchronization, and offline `.satyachar` Avro binary framing helpers.
//!
//! Extracted from [`crate::wasm_client::character_sheet_vault`] so record serialization,
//! `lifepath_config_json` round-tripping, and CRC-64-AVRO `.satyachar` binary encoding/decoding
//! are host-testable without browser `web-sys` APIs.

use crate::wasm_client::character_sheet::{
    AnatomicalZoneState, BoneFractureStatus, CharacterSheetState,
};
use satya_engine::{
    AcuteMentalTraumaTier, ArmorProfileId, HemorrhageGrade, HitLocationId, MeleeWeaponId,
    MoraleState, PhysiologicalStatusCondition, SkillAccessTier, SocialWoundKind,
    aptitudes_from_record, aptitudes_to_record, characteristics_from_record,
    characteristics_to_record, verify_character_sheet_legality,
};
use satya_schema::{
    AnatomicalZoneRecord, CharacterEventType, CharacterSheetEvent, CharacterSheetRecord,
    CharacterVerificationStatus, Participant, SchemaRegistry, SessionMode, SessionRole, subjects,
};

impl CharacterSheetState {
    /// Converts this live [`CharacterSheetState`] into the canonical [`CharacterSheetRecord`] for
    /// SQLite storage, `.satyachar` binary framing, and Apache Iggy `character-sheets` ingestion.
    #[must_use]
    pub fn to_character_sheet_record(&self, owner_user_id: &str) -> CharacterSheetRecord {
        let hit_zones = self
            .hit_zones
            .iter()
            .map(|z| AnatomicalZoneRecord {
                location_code: format!("ZONE_{}", z.location as usize),
                wound_tier: i32::from(z.wound_tier),
                bone_status: z.bone_status.code().to_string(),
                bleed_grade: z.bleed_grade.grade_number(),
            })
            .collect();

        let mut rec = CharacterSheetRecord {
            character_id: if self.character_id.trim().is_empty() {
                format!("char-{}", self.preset_id)
            } else {
                self.character_id.clone()
            },
            owner_user_id: owner_user_id.to_string(),
            version: 1,
            character_name: self.character_name.clone(),
            callsign_or_role: self.callsign_or_role.clone(),
            age_years: self.age_years as i32,
            origin_label: self.origin_label.clone(),
            campaign_sqm: i32::from(self.campaign_sqm),
            operational_heat: i32::from(self.operational_heat),
            granted_access_tags: self
                .granted_access_tags
                .iter()
                .map(|t| t.as_str().to_string())
                .collect(),
            stage_history: self.stage_history.clone(),
            characteristics: characteristics_to_record(&self.characteristics),
            aptitudes: aptitudes_to_record(&self.aptitudes),
            skills: self
                .skills
                .iter()
                .map(|(k, &v)| (k.clone(), v as i32))
                .collect(),
            equipped_ranged_ids: self.equipped_ranged_ids.clone(),
            equipped_melee_codes: self
                .equipped_melee
                .iter()
                .map(|&m| format!("{}", m as usize))
                .collect(),
            equipped_armor_codes: self
                .equipped_armor
                .iter()
                .map(|&a| format!("{}", a as usize))
                .collect(),
            field_gear_ids: self.field_gear.iter().map(|g| g.id.clone()).collect(),
            hit_zones,
            fatigue_level: self.fatigue_level,
            exertion_marks: self.exertion_marks,
            wound_shock_index: self.wound_shock_index,
            blood_units_lost: self.blood_units_lost,
            active_status_conditions: self
                .active_status_conditions
                .iter()
                .map(|&c| c.code().to_string())
                .collect(),
            morale_state: self.morale_state.as_str().to_string(),
            mental_trauma: self.mental_trauma.map(|mt| mt.code().to_string()),
            social_composure_lost: self.social_composure_lost,
            social_wound: self.social_wound.map(|sw| sw.code().to_string()),
            psyche_drift_marks: self.psyche_drift_marks,
            notes: Some(self.notes.clone()),
            lifepath_config_json: self.lifepath_config_json.clone(),
            assets: self.assets.clone(),
            verification_status: self.verification_status,
            sp_budget_valid: true,
            dag_prereqs_valid: true,
            audit_violations: self.audit_violations.clone(),
            content_sha256: String::new(),
            server_signature_hex: None,
            updated_at_us: 0,
        };
        let audit = verify_character_sheet_legality(&rec);
        rec.sp_budget_valid = audit.sp_budget_valid;
        rec.dag_prereqs_valid = audit.dag_prereqs_valid;
        rec.audit_violations = audit.violations;
        rec.verification_status = if !audit.is_legal {
            CharacterVerificationStatus::TamperedOrOverBudget
        } else if rec.server_signature_hex.is_some() {
            CharacterVerificationStatus::VerifiedOfficial
        } else {
            CharacterVerificationStatus::LegalCustom
        };
        rec
    }

    /// Hydrates this [`CharacterSheetState`] from an imported or vault-loaded [`CharacterSheetRecord`].
    pub fn apply_character_sheet_record(&mut self, rec: &CharacterSheetRecord) {
        self.character_id = rec.character_id.clone();
        self.character_name = rec.character_name.clone();
        self.callsign_or_role = rec.callsign_or_role.clone();
        self.age_years = rec.age_years.max(1) as u32;
        self.origin_label = rec.origin_label.clone();
        self.campaign_sqm = rec.campaign_sqm.clamp(0, 10) as u8;
        self.operational_heat = rec.operational_heat.clamp(0, 10) as u8;
        self.granted_access_tags = rec
            .granted_access_tags
            .iter()
            .filter_map(|c| SkillAccessTier::from_code_str(c))
            .collect();
        self.stage_history = rec.stage_history.clone();
        self.characteristics = characteristics_from_record(&rec.characteristics);
        self.aptitudes = aptitudes_from_record(&rec.aptitudes);
        self.skills = rec
            .skills
            .iter()
            .map(|(k, &v)| (k.clone(), v.max(0) as usize))
            .collect();
        self.equipped_ranged_ids = rec.equipped_ranged_ids.clone();
        self.equipped_melee = rec
            .equipped_melee_codes
            .iter()
            .filter_map(|c| {
                c.parse::<usize>()
                    .ok()
                    .and_then(|idx| MeleeWeaponId::ALL.get(idx).copied())
            })
            .collect();
        self.equipped_armor = rec
            .equipped_armor_codes
            .iter()
            .filter_map(|c| {
                c.parse::<usize>()
                    .ok()
                    .and_then(|idx| ArmorProfileId::ALL.get(idx).copied())
            })
            .collect();
        let gear_refs: Vec<&str> = rec.field_gear_ids.iter().map(String::as_str).collect();
        self.field_gear = Self::gear_from_ids(&gear_refs);
        self.notes = rec.notes.clone().unwrap_or_default();
        self.lifepath_config_json = rec.lifepath_config_json.clone();
        self.verification_status = rec.verification_status;
        self.audit_violations = rec.audit_violations.clone();
        self.assets = rec.assets.clone();

        self.fatigue_level = rec.fatigue_level.clamp(0, 5);
        self.exertion_marks = rec.exertion_marks.max(0);
        self.wound_shock_index = rec.wound_shock_index.clamp(0, 5);
        self.blood_units_lost = rec.blood_units_lost.clamp(0.0, 10.0);
        if let Some(m) = MoraleState::from_code_str(&rec.morale_state) {
            self.morale_state = m;
        }
        self.mental_trauma = rec
            .mental_trauma
            .as_deref()
            .and_then(AcuteMentalTraumaTier::from_code_str);
        self.social_composure_lost = rec.social_composure_lost.max(0);
        self.social_wound = rec
            .social_wound
            .as_deref()
            .and_then(SocialWoundKind::from_code_str);
        self.psyche_drift_marks = rec.psyche_drift_marks.clamp(0, 10);
        self.active_status_conditions = rec
            .active_status_conditions
            .iter()
            .filter_map(|c| PhysiologicalStatusCondition::from_code_str(c))
            .collect();

        self.hit_zones = HitLocationId::ALL.map(AnatomicalZoneState::new_healthy);
        for (idx, z_rec) in rec.hit_zones.iter().enumerate() {
            let target_idx = z_rec
                .location_code
                .strip_prefix("ZONE_")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(idx);
            if let Some(zone) = self.hit_zones.get_mut(target_idx) {
                zone.wound_tier = z_rec.wound_tier.clamp(0, 5) as u8;
                zone.bone_status = BoneFractureStatus::from_code(&z_rec.bone_status);
                zone.bleed_grade = HemorrhageGrade::from_grade_number(z_rec.bleed_grade);
            }
        }
    }
}

/// Encodes a [`CharacterSheetRecord`] into a framed `.satyachar` Avro binary payload
/// (`Schema ID 11`, CRC-64-AVRO single-object framing) for offline local export.
pub fn encode_offline_satyachar_bytes(
    record: &CharacterSheetRecord,
    session_id: Option<&str>,
) -> Result<Vec<u8>, String> {
    let registry = SchemaRegistry::new_with_builtin_schemas().map_err(|e| e.to_string())?;
    let sid = session_id
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("local-export");
    let metadata = registry
        .build_metadata(1, subjects::CHARACTER_SHEET, 1, sid, SessionMode::Ttrpg)
        .map_err(|e| e.to_string())?;
    let mut actor = Participant::human(
        &record.owner_user_id,
        &record.owner_user_id,
        &record.character_name,
        SessionRole::Player,
    );
    actor.character_id = Some(record.character_id.clone());
    let event = CharacterSheetEvent {
        metadata,
        character_event_type: CharacterEventType::CharacterExported,
        actor,
        character: record.clone(),
    };
    registry
        .encode_framed(subjects::CHARACTER_SHEET, 1, &event)
        .map_err(|e| e.to_string())
}

/// Decodes a framed `.satyachar` Avro binary payload and runs [`verify_character_sheet_legality`]
/// anti-cheat validation on the embedded [`CharacterSheetRecord`].
pub fn decode_offline_satyachar_bytes(bytes: &[u8]) -> Result<CharacterSheetRecord, String> {
    let registry = SchemaRegistry::new_with_builtin_schemas().map_err(|e| e.to_string())?;
    let (_, event) = registry
        .decode_framed::<CharacterSheetEvent>(bytes, None)
        .map_err(|e| e.to_string())?;
    let mut rec = event.character;
    let report = verify_character_sheet_legality(&rec);
    rec.sp_budget_valid = report.sp_budget_valid;
    rec.dag_prereqs_valid = report.dag_prereqs_valid;
    rec.audit_violations = report.violations;
    rec.verification_status = if report.is_legal {
        CharacterVerificationStatus::LegalCustom
    } else {
        CharacterVerificationStatus::TamperedOrOverBudget
    };
    Ok(rec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wasm_client::character_sheet::apply_wizard_to_character_sheet;
    use crate::wasm_client::state::SatyaWasmState;

    #[test]
    fn test_character_sheet_bleed_progression_and_exertion_rollover() {
        let mut sheet = CharacterSheetState::from_preset("mateo_vance");
        assert_eq!(sheet.blood_units_lost, 0.0);
        assert_eq!(sheet.fatigue_level, 0);
        assert_eq!(sheet.exertion_marks, 0);

        // Apply Grade 4 Arterial Severance (1.0 Unit per 4 Ticks) to Thigh/Femoral and advance +20t
        let thigh_idx = HitLocationId::ThighFemur as usize;
        sheet.apply_zone_mutation(thigh_idx, "bleed_grade", "4");
        assert_eq!(
            sheet.hit_zones[thigh_idx].bleed_grade,
            HemorrhageGrade::Grade4ArterialSeverance
        );
        sheet.apply_status_mutation("advance_bleed_ticks", "20");
        assert!((sheet.blood_units_lost - 5.0).abs() < 1e-6);

        sheet.apply_zone_mutation(0, "stabilize_bleeds", "");
        assert!(sheet.active_bleeds().is_empty());
        sheet.apply_zone_mutation(0, "clear_all", "");
        assert_eq!(sheet.blood_units_lost, 0.0);

        // Accumulate FT + 2 exertion marks -> rolls over into +1 Fatigue Level with 2 remaining marks
        let ft = sheet.characteristics.compute_derived(0).ft_fatigue_threshold;
        sheet.apply_status_mutation("exertion_delta", &(ft + 2).to_string());
        assert_eq!(sheet.fatigue_level, 1);
        assert_eq!(sheet.exertion_marks, 2);

        // Social wound canonical delta_int check (Doubt = +1, Strain = +2, Cognitive Dissonance = +4)
        let base_dint = sheet.compute_combined_condition_impact().delta_int;
        sheet.apply_status_mutation("social_wound", "DOUBT");
        assert_eq!(
            sheet.compute_combined_condition_impact().delta_int,
            base_dint + 1
        );
        sheet.apply_status_mutation("social_wound", "DISSONANCE");
        assert_eq!(
            sheet.compute_combined_condition_impact().delta_int,
            base_dint + 4
        );
    }

    #[test]
    fn test_character_sheet_equipment_mutations_update_encumbrance() {
        let mut sheet = CharacterSheetState::from_preset("priya_nair");
        let base_mass_kg = sheet.compute_encumbrance().raw_carried_mass_kg;

        // Benelli M4 (4.25 kg) + Riot Shield/Staff idx 5 (3.40 kg) + EVA Thruster Pack (4.50 kg) = +12.15 kg
        sheet.apply_equipment_mutation("add_ranged", "benelli_m4_12ga");
        sheet.apply_equipment_mutation("add_melee", "5");
        sheet.apply_equipment_mutation("add_gear", "eva_mag_thruster_pack");
        let loaded_mass_kg = sheet.compute_encumbrance().raw_carried_mass_kg;
        assert!((loaded_mass_kg - (base_mass_kg + 12.15)).abs() < 1e-6);

        // Removing the same three items restores exact baseline carried mass
        sheet.apply_equipment_mutation("remove_ranged", "benelli_m4_12ga");
        sheet.apply_equipment_mutation("remove_melee", "5");
        sheet.apply_equipment_mutation("remove_gear", "eva_mag_thruster_pack");
        assert!((sheet.compute_encumbrance().raw_carried_mass_kg - base_mass_kg).abs() < 1e-6);
    }

    #[test]
    fn test_wizard_handoff_lifepath_json_and_offline_satyachar_roundtrip() {
        let mut st = SatyaWasmState::default();
        st.wizard.character_name = "Kira \"Ghost\" Solis!".to_string();
        apply_wizard_to_character_sheet(&mut st);

        assert_eq!(st.character_sheet.character_id, "char-kira-ghost-solis");
        let lp_json = st
            .character_sheet
            .lifepath_config_json
            .clone()
            .expect("apply_wizard_to_character_sheet must serialize lifepath_config_json");
        assert!(lp_json.contains("origin_roll_2d20"));

        st.character_sheet
            .apply_status_mutation("toggle_condition", "TINNITUS");
        st.character_sheet
            .apply_status_mutation("mental_trauma", "TIER_2");

        let record = st.character_sheet.to_character_sheet_record("usr-kira");
        assert_eq!(record.lifepath_config_json.as_deref(), Some(lp_json.as_str()));

        let bytes = encode_offline_satyachar_bytes(&record, Some("sess-offline-1"))
            .expect("offline .satyachar encode must succeed");
        assert!(!bytes.is_empty());

        let decoded = decode_offline_satyachar_bytes(&bytes)
            .expect("offline .satyachar decode must succeed");
        assert_eq!(decoded.character_name, "Kira \"Ghost\" Solis!");
        assert_eq!(decoded.lifepath_config_json.as_deref(), Some(lp_json.as_str()));
        assert_eq!(
            decoded.verification_status,
            CharacterVerificationStatus::LegalCustom
        );

        let mut restored = CharacterSheetState::default();
        restored.apply_character_sheet_record(&decoded);
        assert_eq!(restored.character_id, "char-kira-ghost-solis");
        assert_eq!(
            restored.lifepath_config_json.as_deref(),
            Some(lp_json.as_str())
        );
        assert_eq!(
            restored.active_status_conditions,
            vec![PhysiologicalStatusCondition::Tinnitus]
        );
        assert_eq!(
            restored.mental_trauma,
            Some(AcuteMentalTraumaTier::Tier2PanicFightOrFlight)
        );
    }
}
