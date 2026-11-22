//! Canonical PyO3 bindings for [`satya_schema`] (`SchemaRegistry`, flexible DTO
//! normalization, typed Serde DTO validation, and wire-enum code catalogs).

use std::collections::BTreeMap;

use pyo3::prelude::*;
use satya_schema::{
    AiGmOperatingMode, AiGmTurnTriggerRequest, AnatomicalZoneRecord, ChannelContext,
    CharacterAptitudesRecord, CharacterAssetKind, CharacterAssetRef,
    CharacterCharacteristicsRecord, CharacterSheetRecord, CharacterVerificationStatus, CodedEnum,
    CombatTrackerState, CombatantTickEntry, FactionClockRecord, GenerateNpcRequest, GmChatRequest,
    GmMapUpdateRequest, GmMemoryNoteRecord, GmRollRequest, GmSessionLogEntry,
    GmTrajectoryReportRequest, McpToolInvocationRecord, MessageVisibility, MoveMapTokenRequest,
    Participant, PiiFieldDescriptor, PlayerAiChatRequest, PlayerAiDoorRequest,
    PlayerAiRollRequest, RegisteredSchema, SchemaRegistry, SessionMode, SessionNpcEntry,
    SessionRole, SessionWorldLedger, StepAiPlayerRequest, TokenPlacement, TokenVisualSpec,
    UpdateAiRallyRequest, UpdateCombatTrackerRequest, UpdateSessionNpcRequest,
    UpdateWorldLedgerRequest, UpsertGmMemoryRequest,
};
use serde_json::{Map, Value, json};

use super::py_bridge::{PyJsonDict, PyJsonInput, py_to_json_value, to_py_err, to_py_object};

const LIFEPATH_EXTRACT_KEYS: &[&str] = &[
    "youth_hobby",
    "personal_hobbies",
    "formative_events",
    "career_terms",
    "adolescent_life_event_roll_2d20",
    "adolescent_life_event_choice",
    "faction_archetype",
    "faction_id",
];

fn normalize_registry_key(raw: &str) -> String {
    raw.trim().to_ascii_lowercase().replace(['-', '_', ' '], "")
}

fn find_avro_record_fields_in_value(
    node: &Value,
    target_name: Option<&str>,
    inherited_ns: Option<&str>,
) -> Option<Vec<String>> {
    match node {
        Value::Object(obj) => {
            let local_ns = obj.get("namespace").and_then(Value::as_str).or(inherited_ns);
            if let Some(ty) = obj.get("type") {
                if ty.as_str() == Some("record") {
                    let rec_name = obj.get("name").and_then(Value::as_str).unwrap_or("");
                    let full_name = match local_ns {
                        Some(ns) if !ns.is_empty() && !rec_name.is_empty() => {
                            format!("{ns}.{rec_name}")
                        }
                        _ => rec_name.to_string(),
                    };
                    let matches = match target_name {
                        None => true,
                        Some(want) => want == rec_name || want == full_name,
                    };
                    if matches
                        && let Some(fields) = obj.get("fields").and_then(Value::as_array)
                    {
                        return Some(
                            fields
                                .iter()
                                .filter_map(|f| {
                                    f.get("name").and_then(Value::as_str).map(str::to_string)
                                })
                                .collect(),
                        );
                    }
                    if let Some(fields) = obj.get("fields").and_then(Value::as_array) {
                        for field in fields {
                            if let Some(ft) = field.get("type")
                                && let Some(found) =
                                    find_avro_record_fields_in_value(ft, target_name, local_ns)
                            {
                                return Some(found);
                            }
                        }
                    }
                } else if let Some(found) =
                    find_avro_record_fields_in_value(ty, target_name, local_ns)
                {
                    return Some(found);
                }
            }
            for container_key in ["items", "values"] {
                if let Some(sub) = obj.get(container_key)
                    && let Some(found) =
                        find_avro_record_fields_in_value(sub, target_name, local_ns)
                {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(items) => {
            for item in items {
                if let Some(found) =
                    find_avro_record_fields_in_value(item, target_name, inherited_ns)
                {
                    return Some(found);
                }
            }
            None
        }
        _ => None,
    }
}

/// Python wrapper around [`satya_schema::SchemaRegistry`] providing `CRC-64-AVRO` fingerprinting,
/// 15-byte binary wire framing, Avro record inspection, and schema-driven GDPR PII manifest inspection.
#[pyclass(name = "SchemaRegistry")]
pub struct PySchemaRegistry {
    inner: SchemaRegistry,
}

impl PySchemaRegistry {
    fn lookup_schema(&self, subject_or_topic: &str, version: u16) -> PyResult<&RegisteredSchema> {
        if let Ok(reg) = self.inner.get_by_subject_version(subject_or_topic, version) {
            return Ok(reg);
        }
        let needle = subject_or_topic.trim();
        self.inner
            .all_schemas()
            .into_iter()
            .find(|r| {
                r.version == version
                    && (r.topic.eq_ignore_ascii_case(needle)
                        || r.subject.eq_ignore_ascii_case(needle))
            })
            .ok_or_else(|| {
                to_py_err(format!(
                    "Schema not found for subject or topic '{subject_or_topic}' version {version}"
                ))
            })
    }
}

#[pymethods]
impl PySchemaRegistry {
    /// Initializes a [`SchemaRegistry`] populated with all built-in Satya `.avsc` schemas.
    #[new]
    pub fn new() -> PyResult<Self> {
        let inner =
            SchemaRegistry::new_with_builtin_schemas().map_err(|e| to_py_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Returns a list of `(subject, version, fingerprint_hex)` for all registered schemas.
    pub fn list_schemas(&self) -> Vec<(String, u16, String)> {
        let mut out: Vec<(String, u16, String)> = self
            .inner
            .all_schemas()
            .into_iter()
            .map(|s| (s.subject.clone(), s.version, s.fingerprint_hex()))
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
        out
    }

    /// Extracts all custom `satya.*` PII/GDPR field annotations across registered schemas as a JSON string.
    pub fn extract_pii_manifest(&self) -> PyResult<String> {
        let mut all_specs: BTreeMap<String, Vec<PiiFieldDescriptor>> = BTreeMap::new();
        for reg in self.inner.all_schemas() {
            let specs = reg.extract_pii_field_manifest();
            if !specs.is_empty() {
                all_specs.insert(format!("{}@v{}", reg.subject, reg.version), specs);
            }
        }
        serde_json::to_string_pretty(&all_specs).map_err(|e| to_py_err(e.to_string()))
    }

    /// Returns the raw `.avsc` JSON string for `(subject, version)` (accepting either subject or topic name).
    #[pyo3(signature = (subject, version))]
    pub fn get_schema_json(&self, subject: &str, version: u16) -> PyResult<String> {
        Ok(self.lookup_schema(subject, version)?.raw_json.clone())
    }

    /// Extracts ordered field names for an Avro record inside `(subject, version)` (or the root record when `record_name` is omitted).
    #[pyo3(signature = (subject, version, record_name=None))]
    pub fn extract_record_fields(
        &self,
        subject: &str,
        version: u16,
        record_name: Option<&str>,
    ) -> PyResult<Vec<String>> {
        let reg = self.lookup_schema(subject, version)?;
        let root: Value = serde_json::from_str(&reg.raw_json)
            .map_err(|e| to_py_err(format!("Failed to parse Avro JSON for '{subject}': {e}")))?;
        let target = record_name.map(str::trim).filter(|s| !s.is_empty());
        find_avro_record_fields_in_value(&root, target, None).ok_or_else(|| {
            to_py_err(format!(
                "Avro record '{}' not found in '{subject}' v{version}",
                target.unwrap_or("<root>")
            ))
        })
    }
}

fn non_empty_str(map: &Map<String, Value>, key: &str) -> Option<String> {
    map.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn json_number_as_i64(val: &Value) -> Option<i64> {
    val.as_i64()
        .or_else(|| val.as_f64().filter(|f| f.is_finite()).map(|f| f as i64))
}

fn json_number_as_f64(val: &Value) -> Option<f64> {
    val.as_f64().filter(|f| f.is_finite())
}

/// Normalizes a partial or full Python/JSON character payload into a canonical [`CharacterSheetRecord`].
pub(crate) fn parse_flexible_character_record(
    val: Value,
    index: usize,
) -> Result<CharacterSheetRecord, String> {
    let raw = match val {
        Value::Null => Map::new(),
        Value::String(ref s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                Map::new()
            } else {
                let parsed: Value = serde_json::from_str(trimmed)
                    .map_err(|e| format!("Invalid CharacterSheetRecord JSON string: {e}"))?;
                match parsed {
                    Value::Null => Map::new(),
                    Value::Object(map) => map,
                    other => {
                        return Err(format!(
                            "CharacterSheetRecord JSON must be an object, got {other}"
                        ));
                    }
                }
            }
        }
        Value::Object(map) => map,
        other => {
            return Err(format!(
                "CharacterSheetRecord input must be a mapping/object, got {other}"
            ));
        }
    };

    let mut base = serde_json::to_value(CharacterSheetRecord::default())
        .map_err(|e| format!("Failed to serialize default CharacterSheetRecord: {e}"))?;
    let base_obj = base
        .as_object_mut()
        .ok_or_else(|| "Internal CharacterSheetRecord object error".to_string())?;

    let cid = non_empty_str(&raw, "character_id").unwrap_or_else(|| format!("pc-{index}"));
    let owner_uid = non_empty_str(&raw, "owner_user_id").unwrap_or_else(|| cid.clone());
    let cname = non_empty_str(&raw, "character_name").unwrap_or_else(|| cid.clone());
    base_obj.insert("character_id".to_string(), Value::String(cid));
    base_obj.insert("owner_user_id".to_string(), Value::String(owner_uid));
    base_obj.insert("character_name".to_string(), Value::String(cname));

    for key in ["callsign_or_role", "origin_label", "morale_state"] {
        if let Some(v) = non_empty_str(&raw, key) {
            base_obj.insert(key.to_string(), Value::String(v));
        }
    }
    if let Some(sha) = raw.get("content_sha256").and_then(Value::as_str) {
        base_obj.insert(
            "content_sha256".to_string(),
            Value::String(sha.to_string()),
        );
    }

    for key in ["mental_trauma", "social_wound", "notes", "server_signature_hex"] {
        if let Some(v) = raw.get(key) {
            if v.is_null() {
                base_obj.insert(key.to_string(), Value::Null);
            } else if let Some(s) = v.as_str() {
                base_obj.insert(key.to_string(), Value::String(s.to_string()));
            }
        }
    }

    for key in ["faction_id", "faction_archetype"] {
        let normalized = non_empty_str(&raw, key)
            .map(Value::String)
            .unwrap_or(Value::Null);
        base_obj.insert(key.to_string(), normalized);
    }

    let lifepath_val = if let Some(s) = non_empty_str(&raw, "lifepath_config_json") {
        Value::String(s)
    } else if let Some(obj) = raw.get("lifepath_config").and_then(Value::as_object) {
        let serialized = serde_json::to_string(obj)
            .map_err(|e| format!("Failed to serialize lifepath_config: {e}"))?;
        Value::String(serialized)
    } else {
        let mut extracted = Map::new();
        for &k in LIFEPATH_EXTRACT_KEYS {
            if let Some(v) = raw.get(k)
                && !v.is_null()
            {
                extracted.insert(k.to_string(), v.clone());
            }
        }
        if extracted.is_empty() {
            Value::Null
        } else {
            let serialized = serde_json::to_string(&extracted)
                .map_err(|e| format!("Failed to serialize extracted lifepath config: {e}"))?;
            Value::String(serialized)
        }
    };
    base_obj.insert("lifepath_config_json".to_string(), lifepath_val);

    if let Some(raw_chars) = raw.get("characteristics").and_then(Value::as_object)
        && let Some(base_chars) = base_obj
            .get_mut("characteristics")
            .and_then(Value::as_object_mut)
    {
        for (k, v) in raw_chars {
            let lk = k.trim().to_ascii_lowercase();
            let canon = match lk.as_str() {
                "for" | "for_stat" | "str" => "for_stat",
                "mass_kg" => "mas_kg",
                "height_m" => "hgt_m",
                "social_savvy" | "attunement" => "sav",
                "bearing" | "poise" | "demeanor" => "bea",
                "warmth" | "rapport" => "wrm",
                other => other,
            };
            if !base_chars.contains_key(canon) {
                continue;
            }
            if matches!(canon, "hgt_m" | "mas_kg") {
                if let Some(f) = json_number_as_f64(v) {
                    base_chars.insert(canon.to_string(), json!(f));
                }
            } else if let Some(i) = json_number_as_i64(v) {
                base_chars.insert(canon.to_string(), json!(i as i32));
            }
        }
    }

    if let Some(raw_apts) = raw.get("aptitudes").and_then(Value::as_object)
        && let Some(base_apts) = base_obj.get_mut("aptitudes").and_then(Value::as_object_mut)
    {
        for (k, v) in raw_apts {
            let lk = k.trim().to_ascii_lowercase();
            if base_apts.contains_key(&lk)
                && let Some(i) = json_number_as_i64(v)
            {
                base_apts.insert(lk, json!(i as i32));
            }
        }
    }

    if let Some(raw_skills) = raw.get("skills").and_then(Value::as_object) {
        let mut skill_map = Map::new();
        for (k, v) in raw_skills {
            if let Some(i) = json_number_as_i64(v) {
                skill_map.insert(k.clone(), json!(i as i32));
            }
        }
        base_obj.insert("skills".to_string(), Value::Object(skill_map));
    }

    for key in [
        "version",
        "age_years",
        "campaign_sqm",
        "institutional_status",
        "street_cred",
        "debt_tier",
        "operational_heat",
        "exertion_marks",
        "social_composure_lost",
        "psyche_drift_marks",
        "updated_at_us",
    ] {
        if let Some(v) = raw.get(key)
            && let Some(i) = json_number_as_i64(v)
        {
            base_obj.insert(key.to_string(), json!(i));
        }
    }
    if let Some(v) = raw
        .get("fatigue_level")
        .or_else(|| raw.get("fatigue_tier"))
        && let Some(i) = json_number_as_i64(v)
    {
        base_obj.insert("fatigue_level".to_string(), json!(i as i32));
    }
    if let Some(v) = raw
        .get("wound_shock_index")
        .or_else(|| raw.get("wound_shock_tier"))
        && let Some(i) = json_number_as_i64(v)
    {
        base_obj.insert("wound_shock_index".to_string(), json!(i as i32));
    }
    if let Some(f) = raw.get("blood_units_lost").and_then(json_number_as_f64) {
        base_obj.insert("blood_units_lost".to_string(), json!(f));
    }

    for key in ["sp_budget_valid", "dag_prereqs_valid"] {
        if let Some(b) = raw.get(key).and_then(Value::as_bool) {
            base_obj.insert(key.to_string(), Value::Bool(b));
        }
    }

    if let Some(vs) = non_empty_str(&raw, "verification_status") {
        let norm = CharacterVerificationStatus::from_code_str(&vs).as_str();
        base_obj.insert(
            "verification_status".to_string(),
            Value::String(norm.to_string()),
        );
    }

    if let Some(arr) = raw.get("granted_access_tags").and_then(Value::as_array)
        && !arr.is_empty()
    {
        base_obj.insert("granted_access_tags".to_string(), Value::Array(arr.clone()));
    }

    for key in [
        "stage_history",
        "equipped_ranged_ids",
        "equipped_melee_codes",
        "equipped_armor_codes",
        "field_gear_ids",
        "hit_zones",
        "active_status_conditions",
        "audit_violations",
    ] {
        if let Some(arr) = raw.get(key).and_then(Value::as_array) {
            base_obj.insert(key.to_string(), Value::Array(arr.clone()));
        }
    }

    if let Some(arr) = raw.get("assets").and_then(Value::as_array) {
        let normalized_assets: Vec<Value> = arr
            .iter()
            .cloned()
            .map(|mut item| {
                if let Some(obj) = item.as_object_mut()
                    && let Some(kind_str) = obj
                        .get("asset_kind")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                {
                    let canon = CharacterAssetKind::from_code_str(&kind_str).as_str();
                    obj.insert("asset_kind".to_string(), Value::String(canon.to_string()));
                }
                item
            })
            .collect();
        base_obj.insert("assets".to_string(), Value::Array(normalized_assets));
    }

    serde_json::from_value::<CharacterSheetRecord>(base)
        .map_err(|e| format!("Invalid CharacterSheetRecord: {e}"))
}

/// Normalizes a sequence of partial or full character objects into [`Vec<CharacterSheetRecord>`].
pub(crate) fn parse_flexible_character_list(
    val: Value,
) -> Result<Vec<CharacterSheetRecord>, String> {
    match val {
        Value::Null => Ok(Vec::new()),
        Value::Array(items) => items
            .into_iter()
            .enumerate()
            .map(|(idx, item)| parse_flexible_character_record(item, idx))
            .collect(),
        other => Err(format!(
            "Character sheet sequence must be a list/array, got {other}"
        )),
    }
}

/// Normalizes a partial or full Python character mapping/JSON string into a canonical [`CharacterSheetRecord`] dict.
#[pyfunction]
#[pyo3(signature = (raw, index=0))]
pub fn normalize_character_sheet_record(
    py: Python<'_>,
    raw: PyJsonInput<'_>,
    index: usize,
) -> PyResult<PyJsonDict> {
    let val = py_to_json_value(&raw, "CharacterSheetRecord")?;
    let record = parse_flexible_character_record(val, index).map_err(to_py_err)?;
    to_py_object(py, &record)
}

/// Normalizes a flexible Python tactical map dict/JSON string into a canonical [`GmMapUpdateRequest`] dict.
#[pyfunction]
#[pyo3(signature = (map_state))]
pub fn normalize_tactical_map(py: Python<'_>, map_state: PyJsonInput<'_>) -> PyResult<PyJsonDict> {
    let val = py_to_json_value(&map_state, "tactical map")?;
    let map = super::tactical_map_py::parse_flexible_map_value(val).map_err(to_py_err)?;
    to_py_object(py, &map)
}

fn default_ai_gm_turn_trigger_request() -> AiGmTurnTriggerRequest {
    // Exhaustive struct literal guarantees compile-time sync when `AiGmTurnTriggerRequest` changes.
    AiGmTurnTriggerRequest {
        session_id: "default".to_string(),
        session_mode: SessionMode::Ttrpg,
        operating_mode: AiGmOperatingMode::Autonomous,
        human_gm_user_id: None,
        trigger_event_id: "ev-0".to_string(),
        trigger_kind: "CHAT".to_string(),
        trigger_actor: Participant::human("u-0", "player", "Player", SessionRole::Player),
        visibility: MessageVisibility::Public,
        channel_context: Some(ChannelContext::InCharacter),
        context_text: String::new(),
        roster: Vec::new(),
        provider: "ollama".to_string(),
        model_id: "gemma4:12b".to_string(),
        vision_model_id: None,
        temperature: 0.35,
        persona_id: "harsh-reality-arbiter".to_string(),
        base_url: None,
        trace_id: None,
        parent_span_id: None,
        lineage_id: None,
    }
}

macro_rules! schema_dto_registry {
    (
        $( ($dto_name:literal, $Ty:ty $(=> $default:expr)?) ),+ $(,)?
    ) => {
        const SCHEMA_DTO_NAMES: &[&str] = &[
            $($dto_name,)+
        ];

        /// Returns a canonical default-populated Python dict for a supported [`satya_schema`] DTO struct.
        #[pyfunction]
        #[pyo3(signature = (dto_name))]
        pub fn schema_default_record(py: Python<'_>, dto_name: &str) -> PyResult<PyJsonDict> {
            let norm = normalize_registry_key(dto_name);
            $(
                $(
                    if norm == normalize_registry_key($dto_name) {
                        return to_py_object(py, &$default);
                    }
                )?
            )+
            Err(to_py_err(format!(
                "No default factory registered for schema DTO '{dto_name}'"
            )))
        }

        /// Strictly deserializes `payload` against the named [`satya_schema`] Rust DTO and returns the canonical dict.
        #[pyfunction]
        #[pyo3(signature = (dto_name, payload))]
        pub fn validate_schema_dto(
            py: Python<'_>,
            dto_name: &str,
            payload: PyJsonInput<'_>,
        ) -> PyResult<PyJsonDict> {
            let val = py_to_json_value(&payload, dto_name)?;
            let norm = normalize_registry_key(dto_name);
            $(
                if norm == normalize_registry_key($dto_name) {
                    let parsed: $Ty = serde_json::from_value(val)
                        .map_err(|e| to_py_err(format!("Invalid {dto_name}: {e}")))?;
                    return to_py_object(py, &parsed);
                }
            )+
            Err(to_py_err(format!(
                "Unknown schema DTO '{}'; expected one of: {}",
                dto_name,
                SCHEMA_DTO_NAMES.join(", ")
            )))
        }
    };
}

schema_dto_registry! {
    ("AiGmTurnTriggerRequest", AiGmTurnTriggerRequest => default_ai_gm_turn_trigger_request()),
    ("AnatomicalZoneRecord", AnatomicalZoneRecord),
    ("CharacterAptitudesRecord", CharacterAptitudesRecord => CharacterAptitudesRecord::default()),
    ("CharacterAssetRef", CharacterAssetRef),
    ("CharacterCharacteristicsRecord", CharacterCharacteristicsRecord => CharacterCharacteristicsRecord::default()),
    ("CharacterSheetRecord", CharacterSheetRecord => CharacterSheetRecord::default()),
    ("CombatTrackerState", CombatTrackerState => CombatTrackerState::default()),
    ("CombatantTickEntry", CombatantTickEntry),
    ("FactionClockRecord", FactionClockRecord),
    ("GenerateNpcRequest", GenerateNpcRequest),
    ("GmChatRequest", GmChatRequest),
    ("GmMapUpdateRequest", GmMapUpdateRequest => GmMapUpdateRequest::default()),
    ("GmMemoryNoteRecord", GmMemoryNoteRecord),
    ("GmRollRequest", GmRollRequest),
    ("GmSessionLogEntry", GmSessionLogEntry),
    ("GmTrajectoryReportRequest", GmTrajectoryReportRequest),
    ("McpToolInvocationRecord", McpToolInvocationRecord),
    ("MoveMapTokenRequest", MoveMapTokenRequest),
    ("Participant", Participant),
    ("PlayerAiChatRequest", PlayerAiChatRequest),
    ("PlayerAiDoorRequest", PlayerAiDoorRequest),
    ("PlayerAiRollRequest", PlayerAiRollRequest),
    ("SessionNpcEntry", SessionNpcEntry => SessionNpcEntry::default()),
    ("SessionWorldLedger", SessionWorldLedger),
    ("StepAiPlayerRequest", StepAiPlayerRequest => StepAiPlayerRequest::default()),
    ("TokenPlacement", TokenPlacement),
    ("TokenVisualSpec", TokenVisualSpec => TokenVisualSpec::default()),
    ("UpdateAiRallyRequest", UpdateAiRallyRequest => UpdateAiRallyRequest::default()),
    ("UpdateCombatTrackerRequest", UpdateCombatTrackerRequest => UpdateCombatTrackerRequest::default()),
    ("UpdateSessionNpcRequest", UpdateSessionNpcRequest => UpdateSessionNpcRequest::default()),
    ("UpdateWorldLedgerRequest", UpdateWorldLedgerRequest => UpdateWorldLedgerRequest::default()),
    ("UpsertGmMemoryRequest", UpsertGmMemoryRequest),
}

/// Returns the sorted list of canonical [`satya_schema`] DTO struct names supported by [`validate_schema_dto`].
#[pyfunction]
pub fn list_schema_dto_names() -> Vec<String> {
    SCHEMA_DTO_NAMES.iter().map(|s| (*s).to_string()).collect()
}

fn coded_enum_codes<T: CodedEnum>() -> Vec<String> {
    T::ALL.iter().map(|v| v.code().to_string()).collect()
}

macro_rules! wire_enum_registry {
    (
        coded: [ $( ($canonical:literal, $EnumTy:ty) ),+ $(,)? ],
        custom: [ $( ($custom_name:literal, $custom_fn:path) ),* $(,)? ]
    ) => {
        const WIRE_ENUM_NAMES: &[&str] = &[
            $($canonical,)+
            $($custom_name,)*
        ];

        fn lookup_wire_enum_codes(enum_name: &str) -> PyResult<Vec<String>> {
            let norm = normalize_registry_key(enum_name);
            $(
                if norm == normalize_registry_key($canonical) {
                    return Ok(coded_enum_codes::<$EnumTy>());
                }
            )+
            $(
                if norm == normalize_registry_key($custom_name) {
                    return Ok($custom_fn());
                }
            )*
            let valid = WIRE_ENUM_NAMES.join(", ");
            Err(to_py_err(format!(
                "Unknown wire enum '{enum_name}'; expected one of: {valid}"
            )))
        }
    };
}

wire_enum_registry! {
    coded: [
        ("AcousticSurfaceKind", satya_engine::action_catalog_stealth::AcousticSurfaceKind),
        ("AffectiveSteeringMode", satya_campaign::AffectiveSteeringMode),
        ("AiArchitectureParadigm", satya_engine::robotics_and_ai_design::AiArchitectureParadigm),
        ("AiGmOperatingMode", satya_schema::AiGmOperatingMode),
        ("AiGmTurnStatus", satya_schema::AiGmTurnStatus),
        ("AlertnessLevel", satya_engine::action_catalog_stealth::AlertnessLevel),
        ("AptitudeCode", satya_engine::characteristics::AptitudeCode),
        ("Arc3EndingHorizon", satya_campaign::Arc3EndingHorizon),
        ("ArmorCoverageTier", satya_engine::engineering_design::ArmorCoverageTier),
        ("ArmorMaterialArchitecture", satya_engine::engineering_design::ArmorMaterialArchitecture),
        ("AwakenedFactionId", satya_campaign::AwakenedFactionId),
        ("BlackMarketRestrictionClass", satya_engine::economy_gear::BlackMarketRestrictionClass),
        ("BotchRule", satya_engine::dice_probabilities::BotchRule),
        ("BulletConstructionKind", satya_engine::engineering_design::BulletConstructionKind),
        ("CageInterventionAction", satya_campaign::CageInterventionAction),
        ("CampaignArcStage", satya_campaign::CampaignArcStage),
        ("CanonicalActionId", satya_engine::action_catalog_stealth::CanonicalActionId),
        ("CareerTermTrack", satya_engine::lifepath_catalog::CareerTermTrack),
        ("CarryMode", satya_engine::dice_probabilities::CarryMode),
        ("CascadeOutcomeKind", satya_engine::dice_probabilities::CascadeOutcomeKind),
        ("CenterDistanceMode", satya_engine::dice_pools::CenterDistanceMode),
        ("ChannelContext", satya_schema::ChannelContext),
        ("CharacteristicCode", satya_engine::characteristics::CharacteristicCode),
        ("ChassisHitZone", satya_engine::robotics_and_ai_design::ChassisHitZone),
        ("CombatDomain", satya_engine::combat_techniques::CombatDomain),
        ("CombatMoveCategory", satya_engine::combat_techniques::CombatMoveCategory),
        ("CombatMovementGait", satya_engine::environment_vehicles::CombatMovementGait),
        ("CombatStance", satya_schema::CombatStance),
        ("CombatTechniqueId", satya_engine::combat_techniques::CombatTechniqueId),
        ("ConspiracyDiscoveryTier", satya_campaign::ConspiracyDiscoveryTier),
        ("ConspiracyNodeId", satya_campaign::ConspiracyNodeId),
        ("ContestedOutcomeKind", satya_engine::contested_cascade::ContestedOutcomeKind),
        ("ContestedTieRule", satya_engine::contested_cascade::ContestedTieRule),
        ("CorporateLabBrand", satya_campaign::CorporateLabBrand),
        ("CoverGrade", satya_schema::CoverGrade),
        ("CoverIdentityTier", satya_engine::economy_gear::CoverIdentityTier),
        ("CyberAugmentId", satya_engine::robotics_and_ai_design::CyberAugmentId),
        ("DroneChassisClass", satya_engine::robotics_and_ai_design::DroneChassisClass),
        ("DroneModuleId", satya_engine::robotics_and_ai_design::DroneModuleId),
        ("EccmHardeningId", satya_engine::electronic_warfare::EccmHardeningId),
        ("EcmMethodId", satya_engine::electronic_warfare::EcmMethodId),
        ("EconomicTier", satya_schema::EconomicTier),
        ("EdgeSide", satya_schema::EdgeSide),
        ("EwHardwareRole", satya_engine::electronic_warfare::EwHardwareRole),
        ("ExplosiveOrdnanceId", satya_engine::explosives_and_arsenal::ExplosiveOrdnanceId),
        ("FactionArchetypeId", satya_campaign::FactionArchetypeId),
        ("FieldDrugId", satya_engine::environmental_hazards::FieldDrugId),
        ("FormativeLifeEventId", satya_engine::lifepath_hobbies_events::FormativeLifeEventId),
        ("GmLookupSourceKind", satya_schema::GmLookupSourceKind),
        ("GmMemoryCategory", satya_schema::GmMemoryCategory),
        ("HighSchoolTrack", satya_engine::lifepath_catalog::HighSchoolTrack),
        ("LogLevel", satya_schema::LogLevel),
        ("MapContinuationDir", satya_schema::MapContinuationDir),
        ("MapScalePreset", satya_schema::MapScalePreset),
        ("MarginAggregation", satya_engine::dice_probabilities::MarginAggregation),
        ("MaterialClass", satya_schema::MaterialClass),
        ("McpServerTier", satya_schema::McpServerTier),
        ("MeleeChassisClass", satya_engine::engineering_design::MeleeChassisClass),
        ("MeleeErgonomicTuning", satya_engine::engineering_design::MeleeErgonomicTuning),
        ("MeleeMaterialGrade", satya_engine::engineering_design::MeleeMaterialGrade),
        ("MessageVisibility", satya_schema::MessageVisibility),
        ("MoraleState", satya_engine::combat::MoraleState),
        ("NetworkIntrusionPhase", satya_engine::electronic_warfare::NetworkIntrusionPhase),
        ("NonFirearmWeaponId", satya_engine::explosives_and_arsenal::NonFirearmWeaponId),
        ("NpcTemplateKind", satya_campaign::NpcTemplateKind),
        ("ObserverAttentionState", satya_engine::action_catalog_stealth::ObserverAttentionState),
        ("OperationalHeatTier", satya_engine::economy_gear::OperationalHeatTier),
        ("OutcomeTier", satya_schema::OutcomeTier),
        ("ParticipantKind", satya_schema::ParticipantKind),
        ("PersonalHobbyId", satya_engine::lifepath_hobbies_events::PersonalHobbyId),
        ("PoliticalEconomyTrigger", satya_campaign::PoliticalEconomyTrigger),
        ("PolycrisisGridMode", satya_campaign::PolycrisisGridMode),
        ("ProjectClockComplexity", satya_engine::engineering_design::ProjectClockComplexity),
        ("PropKind", satya_schema::PropKind),
        ("PrototypeQuirkId", satya_engine::engineering_design::PrototypeQuirkId),
        ("PsycheTraitId", satya_engine::psyche_social::PsycheTraitId),
        ("RuralYouthArchetype", satya_engine::lifepath_catalog::RuralYouthArchetype),
        ("SafehouseTier", satya_engine::economy_gear::SafehouseTier),
        ("SatyangPillarId", satya_campaign::SatyangPillarId),
        ("SectorPreset", satya_maps::tactical_map::SectorPreset),
        ("SessionMode", satya_schema::SessionMode),
        ("SessionRole", satya_schema::SessionRole),
        ("ShotgunLoadId", satya_engine::explosives_and_arsenal::ShotgunLoadId),
        ("SkillAccessTier", satya_engine::skills_trees::SkillAccessTier),
        ("SkillCategory", satya_engine::skills_trees::SkillCategory),
        ("SkillTreeId", satya_engine::skills_trees::SkillTreeId),
        ("SkillType", satya_engine::skills_trees::SkillType),
        ("SocialOriginHookKind", satya_campaign::SocialOriginHookKind),
        ("SocioeconomicClass", satya_engine::economy_gear::SocioeconomicClass),
        ("Stage4Path", satya_engine::lifepath_catalog::Stage4Path),
        ("StoreArchetype", satya_schema::StoreArchetype),
        ("StreetGangArchetype", satya_engine::lifepath_catalog::StreetGangArchetype),
        ("SubstrateCounterplayAction", satya_campaign::SubstrateCounterplayAction),
        ("SurfaceFeatureKind", satya_schema::SurfaceFeatureKind),
        ("SurfaceShapeKind", satya_schema::SurfaceShapeKind),
        ("SurfaceTractionTier", satya_engine::environmental_hazards::SurfaceTractionTier),
        ("SurveillanceNodeKind", satya_engine::electronic_warfare::SurveillanceNodeKind),
        ("SyntheticInteriorityProfile", satya_campaign::SyntheticInteriorityProfile),
        ("SyntheticMetabolicMode", satya_engine::robotics_and_ai_design::SyntheticMetabolicMode),
        ("TacticalOpticOrGearId", satya_engine::explosives_and_arsenal::TacticalOpticOrGearId),
        ("TacticalTileKind", satya_schema::TacticalTileKind),
        ("ToolDependency", satya_engine::task_resolution::ToolDependency),
        ("TradecraftChannelKind", satya_campaign::TradecraftChannelKind),
        ("VehicleClass", satya_engine::environment_vehicles::VehicleClass),
        ("VehicleId", satya_engine::environment_vehicles::VehicleId),
        ("WallEdgeKind", satya_schema::WallEdgeKind),
        ("WeaponDamageType", satya_engine::ranged_combat::WeaponDamageType),
        ("WorkshopFacilityTier", satya_engine::engineering_design::WorkshopFacilityTier),
        ("ZoningContext", satya_schema::ZoningContext),
    ],
    custom: [
        ("NonSolidPropKind", super::tactical_map_py::non_solid_prop_kind_codes),
    ]
}

/// Returns canonical wire codes from [`CodedEnum::ALL`] (or `PROP_CATALOG`) for the requested schema/engine/map/campaign enum.
#[pyfunction]
#[pyo3(signature = (enum_name))]
pub fn list_wire_enum_codes(enum_name: &str) -> PyResult<Vec<String>> {
    lookup_wire_enum_codes(enum_name)
}

/// Returns the sorted list of canonical wire/coded enum names supported by [`list_wire_enum_codes`].
#[pyfunction]
pub fn list_wire_enum_names() -> Vec<String> {
    let mut names: Vec<String> = WIRE_ENUM_NAMES.iter().map(|s| (*s).to_string()).collect();
    names.sort();
    names
}

/// Registers [`PySchemaRegistry`] and `satya-schema` DTO normalization/validation helpers on `satya_py._native`.
pub fn register_schema_functions(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySchemaRegistry>()?;
    m.add_function(wrap_pyfunction!(normalize_character_sheet_record, m)?)?;
    m.add_function(wrap_pyfunction!(normalize_tactical_map, m)?)?;
    m.add_function(wrap_pyfunction!(schema_default_record, m)?)?;
    m.add_function(wrap_pyfunction!(validate_schema_dto, m)?)?;
    m.add_function(wrap_pyfunction!(list_schema_dto_names, m)?)?;
    m.add_function(wrap_pyfunction!(list_wire_enum_codes, m)?)?;
    m.add_function(wrap_pyfunction!(list_wire_enum_names, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::py_bridge::py_to_value;
    use pyo3::types::PyString;

    #[test]
    fn parse_flexible_character_handles_aliases_and_defaults() {
        let partial = json!({
            "character_id": "pc-linnea",
            "characteristics": {
                "for": 13,
                "str": 14,
                "mass_kg": 61.5,
                "trs": 4,
                "cps": 17
            },
            "aptitudes": {
                "apt_qnt": 2
            },
            "fatigue_tier": 2,
            "wound_shock_tier": 1,
            "youth_hobby": "CITIZEN_OSINT_DEEPFAKE",
            "faction_id": "",
            "faction_archetype": "SATYANG_MANJUSRI",
            "notes": null
        });
        let rec = parse_flexible_character_record(partial, 3).unwrap();
        assert_eq!(rec.character_id, "pc-linnea");
        assert_eq!(rec.owner_user_id, "pc-linnea");
        assert_eq!(rec.character_name, "pc-linnea");
        assert_eq!(rec.characteristics.for_stat, 14);
        assert!((rec.characteristics.mas_kg - 61.5).abs() < 1e-6);
        assert_eq!(rec.characteristics.trs, 4);
        assert_eq!(rec.characteristics.cps, 17);
        assert_eq!(rec.aptitudes.apt_qnt, 2);
        assert_eq!(rec.fatigue_level, 2);
        assert_eq!(rec.wound_shock_index, 1);
        assert_eq!(rec.faction_id, None);
        assert_eq!(rec.faction_archetype.as_deref(), Some("SATYANG_MANJUSRI"));
        assert!(
            rec.lifepath_config_json
                .as_deref()
                .unwrap_or("")
                .contains("CITIZEN_OSINT_DEEPFAKE")
        );

        let obj_lp = json!({
            "lifepath_config": {"career_terms": ["CLINICAL_MEDIC"]}
        });
        let rec2 = parse_flexible_character_record(obj_lp, 7).unwrap();
        assert_eq!(rec2.character_id, "pc-7");
        assert!(
            rec2.lifepath_config_json
                .as_deref()
                .unwrap_or("")
                .contains("CLINICAL_MEDIC")
        );
    }

    #[test]
    fn schema_py_registry_dto_validation_and_wire_enums_roundtrip() {
        Python::initialize();
        Python::attach(|py| {
            let reg = PySchemaRegistry::new().unwrap();
            let raw = reg.get_schema_json("character-sheets", 3).unwrap();
            assert!(raw.contains("CharacterSheetRecord"));
            let fields = reg
                .extract_record_fields("character-sheets", 3, Some("CharacterSheetRecord"))
                .unwrap();
            assert_eq!(fields.len(), 44);
            assert!(fields.contains(&"faction_archetype".to_string()));
            assert!(fields.contains(&"institutional_status".to_string()));
            assert!(fields.contains(&"street_cred".to_string()));
            assert!(fields.contains(&"debt_tier".to_string()));

            let def_char = schema_default_record(py, "CharacterSheetRecord").unwrap();
            let val_char = validate_schema_dto(
                py,
                "CharacterSheetRecord",
                def_char.bind(py).clone(),
            )
            .unwrap();
            assert_eq!(
                py_to_value(val_char.bind(py)).unwrap()["morale_state"],
                "STEADY"
            );

            let map_obj = PyString::new(
                py,
                r#"{"map_id":"m1","grid_width":10,"grid_height":8,"tokens":[{"token_id":"t1","label":"Vance","x":2,"y":3}]}"#,
            )
            .into_any();
            let norm_map = normalize_tactical_map(py, map_obj).unwrap();
            let map_v = py_to_value(norm_map.bind(py)).unwrap();
            assert_eq!(map_v["tokens"][0]["entity_id"], "t1");

            assert_eq!(list_wire_enum_codes("SectorPreset").unwrap().len(), 29);
            let non_solid = list_wire_enum_codes("NonSolidPropKind").unwrap();
            assert!(non_solid.contains(&"DRONE_DOCK_PAD".to_string()));
            assert!(non_solid.contains(&"RUG".to_string()));
            assert!(!list_schema_dto_names().is_empty());
        });
    }
}
