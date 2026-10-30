//! Schema-driven GDPR PII field manifest inspection, Art. 17 in-place anonymization,
//! and Art. 15/20 DSAR third-party masking powered by `satya.*` custom `.avsc` properties.

use crate::registry::{
    RegisteredSchema, SchemaError, SchemaRegistry, WIRE_HEADER_LEN, WireFrameHeader,
};
use apache_avro::{Schema as AvroSchema, from_avro_datum, from_value, to_avro_datum};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::{BTreeMap, HashMap};
use std::io::Cursor;

/// Descriptor of an Avro field annotated with `satya.*` GDPR/PII custom schema properties.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PiiFieldDescriptor {
    /// Dot-delimited field path within the event record (e.g. `"sender.user_id"`, `"recipients[].username"`, `"body"`).
    pub field_path: String,
    /// Value of `"satya.pii_category"` if present (e.g. `"DIRECT_IDENTIFIER"`, `"USER_CONTENT"`).
    pub pii_category: Option<String>,
    /// Value of `"satya.subject_role"` inherited from the enclosing participant field (`"PRIMARY_ACTOR"` or `"SECONDARY_PARTICIPANT"`).
    pub subject_role: Option<String>,
    /// Value of `"satya.pii_owner"` (e.g. `"PRIMARY_ACTOR"` for root/metadata fields owned by the event's primary actor).
    pub pii_owner: Option<String>,
    /// Whether this field is marked `"satya.gdpr_subject_key": true`.
    pub is_gdpr_subject_key: bool,
    /// Configured `"satya.erasure_action"` (e.g. `"HMAC_TOMBSTONE"`, `"REPLACE_STATIC"`, `"REDACT_TEXT"`, `"SET_NULL"`, `"SET_ZERO"`).
    pub erasure_action: Option<String>,
    /// Static replacement string from `"satya.erasure_replacement"`, if any.
    pub erasure_replacement: Option<String>,
    /// Whether `"satya.dsar_third_party_mask": true` is set for GDPR Art. 15(4) third-party masking.
    pub dsar_third_party_mask: bool,
}

/// Indicates how a GDPR data subject (`target_user_id`) matched an Avro event record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GdprSubjectMatchKind {
    /// The GDPR subject did not appear in this event.
    NoMatch,
    /// The GDPR subject is the event's `PRIMARY_ACTOR` (and optionally also appears in secondary fields).
    PrimaryActor,
    /// The GDPR subject only appears in one or more `SECONDARY_PARTICIPANT` fields/arrays.
    SecondaryParticipantOnly,
}

impl RegisteredSchema {
    /// Recursively extracts all fields annotated with `satya.*` PII/GDPR custom attributes in this schema.
    pub fn extract_pii_field_manifest(&self) -> Vec<PiiFieldDescriptor> {
        let mut named_records = HashMap::new();
        collect_named_records(&self.avro_schema, &mut named_records);
        let mut out = Vec::new();
        walk_schema_for_pii(&self.avro_schema, "", None, &named_records, &mut out);
        out
    }

    /// Applies GDPR Art. 17 in-place anonymization to an `apache_avro::types::Value` driven by the
    /// custom `satya.*` field annotations in this schema.
    ///
    /// - If `target_user_id` matches the event's `PRIMARY_ACTOR` (`actor`, `sender`, `speaker`, `local_participant`, `registered_by`),
    ///   anonymizes that `Participant` **and** applies `satya.erasure_action` to all event/metadata fields with `satya.pii_owner == "PRIMARY_ACTOR"`.
    /// - If `target_user_id` matches a `SECONDARY_PARTICIPANT` (`recipients[]`, `roster_snapshot[]`, `overlapped_participants[]`, `target_participants[]`, `remote_participant`, `target_participant`),
    ///   anonymizes only that matching `Participant` entry (and scrubs `@username`/`display_name` mentions from text fields) without wiping the primary actor's message body or metrics.
    pub fn anonymize_value_for_subject(
        &self,
        value: &mut apache_avro::types::Value,
        target_user_id: &str,
        tombstone_id: &str,
    ) -> GdprSubjectMatchKind {
        let mut named_records = HashMap::new();
        collect_named_records(&self.avro_schema, &mut named_records);
        anonymize_record_value(
            &self.avro_schema,
            value,
            target_user_id,
            tombstone_id,
            &named_records,
        )
    }

    /// Checks whether `target_user_id` appears in `value` (`PRIMARY_ACTOR` or `SECONDARY_PARTICIPANT`),
    /// and if so returns a DSAR-safe copy where other human participants' `satya.dsar_third_party_mask == true`
    /// fields (`user_id`, `username`) are masked per GDPR Art. 15(4).
    pub fn project_dsar_value_for_subject(
        &self,
        value: &apache_avro::types::Value,
        target_user_id: &str,
    ) -> Option<(GdprSubjectMatchKind, apache_avro::types::Value)> {
        let mut named_records = HashMap::new();
        collect_named_records(&self.avro_schema, &mut named_records);
        let match_kind =
            detect_subject_match(&self.avro_schema, value, target_user_id, &named_records);
        if match_kind == GdprSubjectMatchKind::NoMatch {
            return None;
        }
        let mut cloned = value.clone();
        mask_third_party_participants_in_record(
            &self.avro_schema,
            &mut cloned,
            target_user_id,
            &named_records,
        );
        Some((match_kind, cloned))
    }
}

impl SchemaRegistry {
    /// Applies schema-driven GDPR Art. 17 anonymization to a 15-byte wire-framed Avro message,
    /// returning the [`GdprSubjectMatchKind`] and the re-encoded framed bytes (preserving the exact wire header).
    pub fn anonymize_framed_for_subject(
        &self,
        framed_bytes: &[u8],
        target_user_id: &str,
        tombstone_id: &str,
    ) -> Result<(GdprSubjectMatchKind, Vec<u8>), SchemaError> {
        let header = WireFrameHeader::from_bytes(framed_bytes)?;
        let writer_reg = self.get_by_id(header.schema_id)?;

        if writer_reg.fingerprint_crc64 != header.fingerprint_crc64 {
            return Err(SchemaError::FingerprintMismatch {
                expected: writer_reg.fingerprint_hex(),
                actual: hex::encode(header.fingerprint_crc64),
            });
        }

        let mut cursor = Cursor::new(&framed_bytes[WIRE_HEADER_LEN..]);
        let mut avro_value = from_avro_datum(&writer_reg.avro_schema, &mut cursor, None)?;
        let match_kind =
            writer_reg.anonymize_value_for_subject(&mut avro_value, target_user_id, tombstone_id);

        if match_kind == GdprSubjectMatchKind::NoMatch {
            return Ok((GdprSubjectMatchKind::NoMatch, framed_bytes.to_vec()));
        }

        let avro_bytes = to_avro_datum(&writer_reg.avro_schema, avro_value)?;
        let mut out = Vec::with_capacity(WIRE_HEADER_LEN + avro_bytes.len());
        out.extend_from_slice(&header.to_bytes());
        out.extend_from_slice(&avro_bytes);
        Ok((match_kind, out))
    }

    /// Inspects a 15-byte wire-framed Avro message for GDPR Art. 15/20 DSAR export, returning
    /// `Some((match_kind, masked_event))` with third-party human identifiers masked if `target_user_id` is involved.
    pub fn export_dsar_framed_for_subject<T: DeserializeOwned>(
        &self,
        framed_bytes: &[u8],
        target_user_id: &str,
    ) -> Result<Option<(GdprSubjectMatchKind, T)>, SchemaError> {
        let header = WireFrameHeader::from_bytes(framed_bytes)?;
        let writer_reg = self.get_by_id(header.schema_id)?;

        if writer_reg.fingerprint_crc64 != header.fingerprint_crc64 {
            return Err(SchemaError::FingerprintMismatch {
                expected: writer_reg.fingerprint_hex(),
                actual: hex::encode(header.fingerprint_crc64),
            });
        }

        let mut cursor = Cursor::new(&framed_bytes[WIRE_HEADER_LEN..]);
        let avro_value = from_avro_datum(&writer_reg.avro_schema, &mut cursor, None)?;
        let Some((match_kind, masked_val)) =
            writer_reg.project_dsar_value_for_subject(&avro_value, target_user_id)
        else {
            return Ok(None);
        };
        let decoded: T = from_value(&masked_val)?;
        Ok(Some((match_kind, decoded)))
    }
}

fn collect_named_records<'a>(
    schema: &'a AvroSchema,
    map: &mut HashMap<String, &'a apache_avro::schema::RecordSchema>,
) {
    match schema {
        AvroSchema::Record(rec) => {
            map.insert(rec.name.fullname(None), rec);
            map.insert(rec.name.name.clone(), rec);
            for f in &rec.fields {
                collect_named_records(&f.schema, map);
            }
        }
        AvroSchema::Array(arr) => collect_named_records(&arr.items, map),
        AvroSchema::Map(m) => collect_named_records(&m.types, map),
        AvroSchema::Union(u) => {
            for v in u.variants() {
                collect_named_records(v, map);
            }
        }
        _ => {}
    }
}

fn attr_str(attrs: &BTreeMap<String, serde_json::Value>, key: &str) -> Option<String> {
    attrs
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn attr_bool(attrs: &BTreeMap<String, serde_json::Value>, key: &str) -> bool {
    attrs.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
}

fn walk_schema_for_pii<'a>(
    schema: &'a AvroSchema,
    prefix: &str,
    inherited_subject_role: Option<&str>,
    named_records: &HashMap<String, &'a apache_avro::schema::RecordSchema>,
    out: &mut Vec<PiiFieldDescriptor>,
) {
    match schema {
        AvroSchema::Record(rec) => {
            for field in &rec.fields {
                let path = if prefix.is_empty() {
                    field.name.clone()
                } else {
                    format!("{prefix}.{}", field.name)
                };
                let role_override = attr_str(&field.custom_attributes, "satya.subject_role");
                let active_role = role_override.as_deref().or(inherited_subject_role);

                let pii_category = attr_str(&field.custom_attributes, "satya.pii_category");
                let pii_owner = attr_str(&field.custom_attributes, "satya.pii_owner");
                let is_key = attr_bool(&field.custom_attributes, "satya.gdpr_subject_key");
                let erasure_action = attr_str(&field.custom_attributes, "satya.erasure_action");
                let erasure_replacement =
                    attr_str(&field.custom_attributes, "satya.erasure_replacement");
                let dsar_mask = attr_bool(&field.custom_attributes, "satya.dsar_third_party_mask");

                if pii_category.is_some() || is_key || erasure_action.is_some() || dsar_mask {
                    out.push(PiiFieldDescriptor {
                        field_path: path.clone(),
                        pii_category,
                        subject_role: active_role.map(|s| s.to_string()),
                        pii_owner,
                        is_gdpr_subject_key: is_key,
                        erasure_action,
                        erasure_replacement,
                        dsar_third_party_mask: dsar_mask,
                    });
                }

                walk_field_schema_for_pii(&field.schema, &path, active_role, named_records, out);
            }
        }
        AvroSchema::Ref { name } => {
            let full = name.fullname(None);
            if let Some(rec) = named_records
                .get(&full)
                .or_else(|| named_records.get(&name.name))
            {
                for field in &rec.fields {
                    let path = if prefix.is_empty() {
                        field.name.clone()
                    } else {
                        format!("{prefix}.{}", field.name)
                    };
                    let pii_category = attr_str(&field.custom_attributes, "satya.pii_category");
                    let pii_owner = attr_str(&field.custom_attributes, "satya.pii_owner");
                    let is_key = attr_bool(&field.custom_attributes, "satya.gdpr_subject_key");
                    let erasure_action = attr_str(&field.custom_attributes, "satya.erasure_action");
                    let erasure_replacement =
                        attr_str(&field.custom_attributes, "satya.erasure_replacement");
                    let dsar_mask =
                        attr_bool(&field.custom_attributes, "satya.dsar_third_party_mask");

                    if pii_category.is_some() || is_key || erasure_action.is_some() || dsar_mask {
                        out.push(PiiFieldDescriptor {
                            field_path: path,
                            pii_category,
                            subject_role: inherited_subject_role.map(|s| s.to_string()),
                            pii_owner,
                            is_gdpr_subject_key: is_key,
                            erasure_action,
                            erasure_replacement,
                            dsar_third_party_mask: dsar_mask,
                        });
                    }
                }
            }
        }
        _ => {}
    }
}

fn walk_field_schema_for_pii<'a>(
    schema: &'a AvroSchema,
    path: &str,
    active_role: Option<&str>,
    named_records: &HashMap<String, &'a apache_avro::schema::RecordSchema>,
    out: &mut Vec<PiiFieldDescriptor>,
) {
    match schema {
        AvroSchema::Record(_) | AvroSchema::Ref { .. } => {
            walk_schema_for_pii(schema, path, active_role, named_records, out);
        }
        AvroSchema::Array(arr) => {
            let arr_path = format!("{path}[]");
            walk_field_schema_for_pii(&arr.items, &arr_path, active_role, named_records, out);
        }
        AvroSchema::Union(u) => {
            for v in u.variants() {
                if !matches!(v, AvroSchema::Null) {
                    walk_field_schema_for_pii(v, path, active_role, named_records, out);
                }
            }
        }
        _ => {}
    }
}

fn resolve_record_schema<'a>(
    schema: &'a AvroSchema,
    named_records: &HashMap<String, &'a apache_avro::schema::RecordSchema>,
) -> Option<&'a apache_avro::schema::RecordSchema> {
    match schema {
        AvroSchema::Record(rec) => Some(rec),
        AvroSchema::Ref { name } => {
            let full = name.fullname(None);
            named_records
                .get(&full)
                .or_else(|| named_records.get(&name.name))
                .copied()
        }
        AvroSchema::Union(u) => u
            .variants()
            .iter()
            .find_map(|v| resolve_record_schema(v, named_records)),
        _ => None,
    }
}

fn participant_record_matches_subject(
    rec_schema: &apache_avro::schema::RecordSchema,
    fields: &[(String, apache_avro::types::Value)],
    target_user_id: &str,
) -> Option<(String, String)> {
    let mut matched_uid = false;
    let mut username = String::new();
    let mut display_name = String::new();

    for f_def in &rec_schema.fields {
        let Some((_, val)) = fields.iter().find(|(k, _)| k == &f_def.name) else {
            continue;
        };
        if attr_bool(&f_def.custom_attributes, "satya.gdpr_subject_key")
            && let apache_avro::types::Value::String(s) = val
            && s == target_user_id
        {
            matched_uid = true;
        }
        if f_def.name == "username"
            && let apache_avro::types::Value::String(s) = val
        {
            username = s.clone();
        }
        if f_def.name == "display_name"
            && let apache_avro::types::Value::String(s) = val
        {
            display_name = s.clone();
        }
    }

    if matched_uid {
        Some((username, display_name))
    } else {
        None
    }
}

fn anonymize_single_participant_fields(
    rec_schema: &apache_avro::schema::RecordSchema,
    fields: &mut [(String, apache_avro::types::Value)],
    tombstone_id: &str,
) {
    for f_def in &rec_schema.fields {
        let Some(action) = attr_str(&f_def.custom_attributes, "satya.erasure_action") else {
            continue;
        };
        let Some((_, val)) = fields.iter_mut().find(|(k, _)| k == &f_def.name) else {
            continue;
        };
        apply_field_erasure_action(
            val,
            &action,
            attr_str(&f_def.custom_attributes, "satya.erasure_replacement").as_deref(),
            tombstone_id,
        );
    }
}

fn apply_field_erasure_action(
    val: &mut apache_avro::types::Value,
    action: &str,
    replacement: Option<&str>,
    tombstone_id: &str,
) {
    use apache_avro::types::Value;
    match action {
        "HMAC_TOMBSTONE" => {
            if let Value::String(s) = val {
                *s = tombstone_id.to_string();
            }
        }
        "REPLACE_STATIC" | "REDACT_TEXT" => {
            let rep = replacement.unwrap_or("[Redacted]").to_string();
            match val {
                Value::String(s) => *s = rep,
                Value::Union(idx, inner) => {
                    if let Value::String(s) = inner.as_mut() {
                        *s = rep;
                    } else {
                        *idx = 0;
                        **inner = Value::Null;
                    }
                }
                _ => {}
            }
        }
        "SET_NULL" => {
            if let Value::Union(idx, inner) = val {
                *idx = 0;
                **inner = Value::Null;
            }
        }
        "SET_ZERO" => match val {
            Value::Int(n) => *n = 0,
            Value::Long(n) => *n = 0,
            Value::Float(n) => *n = 0.0,
            Value::Double(n) => *n = 0.0,
            _ => {}
        },
        _ => {}
    }
}

fn anonymize_participant_value_if_matches(
    field_schema: &AvroSchema,
    val: &mut apache_avro::types::Value,
    target_user_id: &str,
    tombstone_id: &str,
    named_records: &HashMap<String, &apache_avro::schema::RecordSchema>,
    scrub_terms: &mut Vec<String>,
) -> bool {
    use apache_avro::types::Value;
    match val {
        Value::Record(fields) => {
            if let Some(rec_schema) = resolve_record_schema(field_schema, named_records)
                && let Some((uname, dname)) =
                    participant_record_matches_subject(rec_schema, fields, target_user_id)
            {
                if !uname.is_empty() {
                    scrub_terms.push(uname);
                }
                if !dname.is_empty() {
                    scrub_terms.push(dname);
                }
                anonymize_single_participant_fields(rec_schema, fields, tombstone_id);
                return true;
            }
            false
        }
        Value::Union(_, inner) => anonymize_participant_value_if_matches(
            field_schema,
            inner,
            target_user_id,
            tombstone_id,
            named_records,
            scrub_terms,
        ),
        Value::Array(items) => {
            let elem_schema = match field_schema {
                AvroSchema::Array(arr) => &*arr.items,
                _ => field_schema,
            };
            let mut any = false;
            for item in items.iter_mut() {
                if anonymize_participant_value_if_matches(
                    elem_schema,
                    item,
                    target_user_id,
                    tombstone_id,
                    named_records,
                    scrub_terms,
                ) {
                    any = true;
                }
            }
            any
        }
        _ => false,
    }
}

fn apply_primary_actor_owned_erasure(
    schema: &AvroSchema,
    val: &mut apache_avro::types::Value,
    scrub_terms: &[String],
    tombstone_id: &str,
    named_records: &HashMap<String, &apache_avro::schema::RecordSchema>,
) {
    use apache_avro::types::Value;
    let Some(rec_schema) = resolve_record_schema(schema, named_records) else {
        return;
    };
    let Value::Record(fields) = val else {
        return;
    };

    for f_def in &rec_schema.fields {
        let Some((_, f_val)) = fields.iter_mut().find(|(k, _)| k == &f_def.name) else {
            continue;
        };
        if attr_str(&f_def.custom_attributes, "satya.pii_owner").as_deref() == Some("PRIMARY_ACTOR")
            && let Some(action) = attr_str(&f_def.custom_attributes, "satya.erasure_action")
        {
            if action == "REGEX_SCRUB_SUBJECT" {
                if let Value::String(s) = f_val {
                    for term in scrub_terms {
                        if !term.is_empty() && s.contains(term) {
                            *s = s.replace(term, tombstone_id);
                        }
                    }
                }
            } else {
                apply_field_erasure_action(
                    f_val,
                    &action,
                    attr_str(&f_def.custom_attributes, "satya.erasure_replacement").as_deref(),
                    tombstone_id,
                );
            }
        } else if matches!(f_def.schema, AvroSchema::Record(_) | AvroSchema::Ref { .. }) {
            apply_primary_actor_owned_erasure(
                &f_def.schema,
                f_val,
                scrub_terms,
                tombstone_id,
                named_records,
            );
        }
    }
}

fn scrub_secondary_mentions_in_text_fields(
    schema: &AvroSchema,
    val: &mut apache_avro::types::Value,
    scrub_terms: &[String],
    tombstone_id: &str,
    named_records: &HashMap<String, &apache_avro::schema::RecordSchema>,
) {
    use apache_avro::types::Value;
    if scrub_terms.is_empty() {
        return;
    }
    let Some(rec_schema) = resolve_record_schema(schema, named_records) else {
        return;
    };
    let Value::Record(fields) = val else {
        return;
    };

    for f_def in &rec_schema.fields {
        if attr_str(&f_def.custom_attributes, "satya.pii_owner").as_deref() != Some("PRIMARY_ACTOR")
        {
            continue;
        }
        let Some((_, f_val)) = fields.iter_mut().find(|(k, _)| k == &f_def.name) else {
            continue;
        };
        match f_val {
            Value::String(s) => {
                for term in scrub_terms {
                    if !term.is_empty() && s.contains(term) {
                        *s = s.replace(term, tombstone_id);
                    }
                }
            }
            Value::Union(_, inner) => {
                if let Value::String(s) = inner.as_mut() {
                    for term in scrub_terms {
                        if !term.is_empty() && s.contains(term) {
                            *s = s.replace(term, tombstone_id);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn anonymize_record_value(
    schema: &AvroSchema,
    value: &mut apache_avro::types::Value,
    target_user_id: &str,
    tombstone_id: &str,
    named_records: &HashMap<String, &apache_avro::schema::RecordSchema>,
) -> GdprSubjectMatchKind {
    use apache_avro::types::Value;
    let Some(rec_schema) = resolve_record_schema(schema, named_records) else {
        return GdprSubjectMatchKind::NoMatch;
    };
    let Value::Record(fields) = value else {
        return GdprSubjectMatchKind::NoMatch;
    };

    let mut primary_matched = false;
    let mut secondary_matched = false;
    let mut scrub_terms = vec![target_user_id.to_string()];

    for f_def in &rec_schema.fields {
        let Some(role) = attr_str(&f_def.custom_attributes, "satya.subject_role") else {
            continue;
        };
        let Some((_, f_val)) = fields.iter_mut().find(|(k, _)| k == &f_def.name) else {
            continue;
        };
        let matched = anonymize_participant_value_if_matches(
            &f_def.schema,
            f_val,
            target_user_id,
            tombstone_id,
            named_records,
            &mut scrub_terms,
        );
        if matched {
            if role == "PRIMARY_ACTOR" {
                primary_matched = true;
            } else {
                secondary_matched = true;
            }
        }
    }

    if primary_matched {
        apply_primary_actor_owned_erasure(schema, value, &scrub_terms, tombstone_id, named_records);
        GdprSubjectMatchKind::PrimaryActor
    } else if secondary_matched {
        scrub_secondary_mentions_in_text_fields(
            schema,
            value,
            &scrub_terms,
            tombstone_id,
            named_records,
        );
        GdprSubjectMatchKind::SecondaryParticipantOnly
    } else {
        GdprSubjectMatchKind::NoMatch
    }
}

fn detect_subject_match(
    schema: &AvroSchema,
    value: &apache_avro::types::Value,
    target_user_id: &str,
    named_records: &HashMap<String, &apache_avro::schema::RecordSchema>,
) -> GdprSubjectMatchKind {
    let mut probe = value.clone();
    anonymize_record_value(schema, &mut probe, target_user_id, "probe", named_records)
}

fn mask_participant_if_third_party(
    field_schema: &AvroSchema,
    val: &mut apache_avro::types::Value,
    target_user_id: &str,
    named_records: &HashMap<String, &apache_avro::schema::RecordSchema>,
) {
    use apache_avro::types::Value;
    match val {
        Value::Record(fields) => {
            let Some(rec_schema) = resolve_record_schema(field_schema, named_records) else {
                return;
            };
            let is_ai = fields.iter().any(|(k, v)| {
                k == "participant_kind" && matches!(v, Value::Enum(_, sym) if sym == "AI_AGENT")
            });
            if is_ai
                || participant_record_matches_subject(rec_schema, fields, target_user_id).is_some()
            {
                return;
            }
            for f_def in &rec_schema.fields {
                if !attr_bool(&f_def.custom_attributes, "satya.dsar_third_party_mask") {
                    continue;
                }
                let mask_val = attr_str(&f_def.custom_attributes, "satya.dsar_mask_value")
                    .unwrap_or_else(|| "redacted-third-party".to_string());
                if let Some((_, Value::String(s))) =
                    fields.iter_mut().find(|(k, _)| k == &f_def.name)
                {
                    *s = mask_val;
                }
            }
        }
        Value::Union(_, inner) => {
            mask_participant_if_third_party(field_schema, inner, target_user_id, named_records);
        }
        Value::Array(items) => {
            let elem_schema = match field_schema {
                AvroSchema::Array(arr) => &*arr.items,
                _ => field_schema,
            };
            for item in items.iter_mut() {
                mask_participant_if_third_party(elem_schema, item, target_user_id, named_records);
            }
        }
        _ => {}
    }
}

fn mask_third_party_participants_in_record(
    schema: &AvroSchema,
    value: &mut apache_avro::types::Value,
    target_user_id: &str,
    named_records: &HashMap<String, &apache_avro::schema::RecordSchema>,
) {
    use apache_avro::types::Value;
    let Some(rec_schema) = resolve_record_schema(schema, named_records) else {
        return;
    };
    let Value::Record(fields) = value else {
        return;
    };

    for f_def in &rec_schema.fields {
        if attr_str(&f_def.custom_attributes, "satya.subject_role").is_none() {
            continue;
        }
        if let Some((_, f_val)) = fields.iter_mut().find(|(k, _)| k == &f_def.name) {
            mask_participant_if_third_party(&f_def.schema, f_val, target_user_id, named_records);
        }
    }
}
