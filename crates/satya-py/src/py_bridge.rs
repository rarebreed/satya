//! Shared PyO3 0.27 Zero-JSON bridge helpers (`PySerde<T>`, `PyEnum<T>`, `CodedEnum` parsing,
//! native Python dict/list <-> `serde_json::Value` serialization) across `crates/satya-py`.

use std::collections::BTreeMap;

use pyo3::Borrowed;
use pyo3::conversion::FromPyObjectOwned;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{
    PyBool, PyByteArray, PyBytes, PyDict, PyFloat, PyInt, PyList, PyMapping, PyString,
};
use satya_schema::CodedEnum;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Number, Value};

/// Semantic type alias representing a Python `dict[str, Any]` returned from Rust via `to_py_object`.
pub type PyJsonDict = Py<PyAny>;

/// Semantic type alias representing a Python `list[dict[str, Any]]` returned from Rust.
pub type PyJsonList = Py<PyAny>;

/// Semantic type alias for an incoming Python `Mapping[str, Any] | str` boundary parameter.
pub type PyJsonInput<'py> = Bound<'py, PyAny>;

/// Semantic type alias for an incoming Python `Sequence[Any] | str` boundary parameter.
pub type PyListInput<'py> = Bound<'py, PyAny>;

/// Converts any stringifiable error message into a `PyValueError`.
pub fn to_py_err(msg: impl Into<String>) -> PyErr {
    PyValueError::new_err(msg.into())
}

/// Normalizes an enum/code token to `UPPER_SNAKE_CASE`.
#[must_use]
pub fn normalize_code(raw: &str) -> String {
    raw.trim().to_ascii_uppercase().replace(['-', ' '], "_")
}

/// Normalizes legacy `"CONCRETE"` wall/barrier material strings to `"CONCRETE_BLOCK"`.
#[must_use]
pub fn normalize_material_code(raw: &str) -> String {
    let norm = normalize_code(raw);
    if norm == "CONCRETE" {
        "CONCRETE_BLOCK".to_string()
    } else {
        norm
    }
}

/// Parses a `CodedEnum` (case-insensitive, `-`/space tolerant), auto-generating valid codes on error.
pub fn parse_coded_enum<T: CodedEnum>(raw: &str, label: &str) -> PyResult<T> {
    let trimmed = raw.trim();
    let upper = normalize_code(trimmed);
    let lower = upper.to_ascii_lowercase();
    if let Some(val) = T::from_code_str(trimmed)
        .or_else(|| T::from_code_str(&upper))
        .or_else(|| T::from_code_str(&lower))
    {
        return Ok(val);
    }
    let valid: Vec<&str> = T::ALL.iter().map(|v| v.code()).collect();
    let preview = if valid.len() <= 12 {
        valid.join(", ")
    } else {
        format!("{} (and {} more)", valid[..8].join(", "), valid.len() - 8)
    };
    Err(to_py_err(format!(
        "Unknown {label} `{raw}`; expected one of: {preview}"
    )))
}

/// Parses a Serde string enum case-insensitively (`snake_case` / `SCREAMING_SNAKE_CASE` / raw).
pub fn parse_serde_enum_ci<T: DeserializeOwned>(raw: &str, kind_label: &str) -> PyResult<T> {
    let trimmed = raw.trim();
    for candidate in [
        trimmed.to_string(),
        normalize_code(trimmed),
        trimmed.to_ascii_lowercase().replace(['-', ' '], "_"),
    ] {
        if let Ok(parsed) = serde_json::from_value::<T>(Value::String(candidate)) {
            return Ok(parsed);
        }
    }
    Err(to_py_err(format!("Unknown {kind_label} `{raw}`")))
}

/// PyO3 argument extractor for any [`CodedEnum`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PyEnum<T: CodedEnum>(pub T);

impl<'a, 'py, T: CodedEnum> FromPyObject<'a, 'py> for PyEnum<T> {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        let raw: String = obj.extract()?;
        parse_coded_enum::<T>(&raw, std::any::type_name::<T>()).map(Self)
    }
}

/// Converts a [`Value`] directly into a bound Python object (`dict`, `list`, `bool`, `int`, `float`, `str`, `None`).
pub fn value_to_bound_py<'py>(py: Python<'py>, value: &Value) -> PyResult<Bound<'py, PyAny>> {
    match value {
        Value::Null => Ok(py.None().into_bound(py)),
        Value::Bool(b) => Ok(PyBool::new(py, *b).to_owned().into_any()),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.into_pyobject(py)?.into_any())
            } else if let Some(u) = n.as_u64() {
                Ok(u.into_pyobject(py)?.into_any())
            } else if let Some(f) = n.as_f64() {
                Ok(f.into_pyobject(py)?.into_any())
            } else {
                Err(to_py_err("Non-finite numeric value cannot convert to Python"))
            }
        }
        Value::String(s) => Ok(PyString::new(py, s).into_any()),
        Value::Array(arr) => {
            let mut items = Vec::with_capacity(arr.len());
            for item in arr {
                items.push(value_to_bound_py(py, item)?);
            }
            Ok(PyList::new(py, items)?.into_any())
        }
        Value::Object(map) => {
            let dict = PyDict::new(py);
            for (k, v) in map {
                dict.set_item(k, value_to_bound_py(py, v)?)?;
            }
            Ok(dict.into_any())
        }
    }
}

/// Converts any [`Serialize`] value directly into a native Python `Py<PyAny>` without intermediate JSON strings.
pub fn to_py_object<T: Serialize>(py: Python<'_>, value: &T) -> PyResult<Py<PyAny>> {
    let val = serde_json::to_value(value)
        .map_err(|e| to_py_err(format!("Serialization to Python value failed: {e}")))?;
    Ok(value_to_bound_py(py, &val)?.unbind())
}

/// Recursively converts a native Python object (`dict`, `Mapping`, `list`, `tuple`, `bool`, `int`, `float`, `str`, `None`) into [`Value`].
pub fn py_to_value(obj: &Bound<'_, PyAny>) -> PyResult<Value> {
    if obj.is_none() {
        return Ok(Value::Null);
    }
    // Check `PyBool` BEFORE `PyInt` because in Python `isinstance(True, int)` is `True`.
    if obj.is_instance_of::<PyBool>() {
        return Ok(Value::Bool(obj.extract::<bool>()?));
    }
    if obj.is_instance_of::<PyInt>() {
        if let Ok(i) = obj.extract::<i64>() {
            return Ok(Value::Number(Number::from(i)));
        }
        if let Ok(u) = obj.extract::<u64>() {
            return Ok(Value::Number(Number::from(u)));
        }
        let f: f64 = obj.extract()?;
        return Number::from_f64(f)
            .map(Value::Number)
            .ok_or_else(|| to_py_err("Integer overflow converting Python int to numeric Value"));
    }
    if obj.is_instance_of::<PyFloat>() {
        let f: f64 = obj.extract()?;
        return Number::from_f64(f)
            .map(Value::Number)
            .ok_or_else(|| to_py_err(format!("Non-finite float {f} cannot convert to Value")));
    }
    if obj.is_instance_of::<PyString>() {
        return Ok(Value::String(obj.extract::<String>()?));
    }
    if obj.is_instance_of::<PyBytes>() || obj.is_instance_of::<PyByteArray>() {
        return Err(to_py_err("Raw bytes are not supported in structured FFI values"));
    }
    if let Ok(dict) = obj.cast::<PyDict>() {
        let mut map = Map::new();
        for (k, v) in dict.iter() {
            let key: String = k
                .extract()
                .map_err(|_| to_py_err("Mapping keys must be strings"))?;
            map.insert(key, py_to_value(&v)?);
        }
        return Ok(Value::Object(map));
    }
    if let Ok(mapping) = obj.cast::<PyMapping>() {
        let items = mapping.items()?;
        let mut map = Map::new();
        for entry in items.iter() {
            let (k, v): (Bound<'_, PyAny>, Bound<'_, PyAny>) = entry.extract()?;
            let key: String = k
                .extract()
                .map_err(|_| to_py_err("Mapping keys must be strings"))?;
            map.insert(key, py_to_value(&v)?);
        }
        return Ok(Value::Object(map));
    }
    if let Ok(iter) = obj.try_iter() {
        let mut vec = Vec::new();
        for item in iter {
            vec.push(py_to_value(&item?)?);
        }
        return Ok(Value::Array(vec));
    }
    Err(to_py_err(format!(
        "Unsupported Python value type '{}' in Serde FFI input",
        obj.get_type().name()?
    )))
}

/// Extracts either a native Python dict/list OR a JSON-encoded string into a [`Value`].
///
/// Empty/whitespace strings and `None` yield `Value::Null`.
pub fn py_to_json_value(obj: &Bound<'_, PyAny>, context: &str) -> PyResult<Value> {
    if obj.is_none() {
        return Ok(Value::Null);
    }
    if obj.is_instance_of::<PyString>() {
        let raw: String = obj.extract()?;
        if raw.trim().is_empty() {
            return Ok(Value::Null);
        }
        return serde_json::from_str(&raw)
            .map_err(|e| to_py_err(format!("Invalid {context} JSON string: {e}")));
    }
    py_to_value(obj)
}

/// Extracts a `DeserializeOwned` type from a native Python object or JSON string.
pub fn from_py_object<T: DeserializeOwned>(obj: &Bound<'_, PyAny>, context: &str) -> PyResult<T> {
    let val = py_to_json_value(obj, context)?;
    serde_json::from_value(val).map_err(|e| to_py_err(format!("Invalid {context}: {e}")))
}

/// Extracts an optional `DeserializeOwned` type (`None` or blank `str` yields `Ok(None)`).
pub fn opt_from_py_object<T: DeserializeOwned>(
    obj: Option<Bound<'_, PyAny>>,
    context: &str,
) -> PyResult<Option<T>> {
    let Some(bound) = obj else {
        return Ok(None);
    };
    let val = py_to_json_value(&bound, context)?;
    if val.is_null() {
        return Ok(None);
    }
    serde_json::from_value(val)
        .map(Some)
        .map_err(|e| to_py_err(format!("Invalid {context}: {e}")))
}

/// Bidirectional Serde <-> Python wrapper (`FromPyObject` + `IntoPyObject`) requiring zero JSON.
#[derive(Debug, Clone)]
pub struct PySerde<T>(pub T);

impl<'a, 'py, T: DeserializeOwned> FromPyObject<'a, 'py> for PySerde<T> {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        from_py_object(&obj, std::any::type_name::<T>()).map(Self)
    }
}

impl<'py, T: Serialize> IntoPyObject<'py> for PySerde<T> {
    type Target = PyAny;
    type Output = Bound<'py, PyAny>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        let val = serde_json::to_value(&self.0)
            .map_err(|e| to_py_err(format!("PySerde serialize failed: {e}")))?;
        value_to_bound_py(py, &val)
    }
}

/// Helper to build a JSON-like `Value` normalization map for default-overlay inputs (e.g. `CharacterCharacteristics` with `"for"` alias).
pub fn merge_object_defaults<T: Serialize + DeserializeOwned>(
    override_val: Option<&Value>,
    aliases: &[(&str, &str)],
    context: &str,
) -> PyResult<T> {
    let mut base = serde_json::to_value(BTreeMap::<String, Value>::new())
        .map_err(|e| to_py_err(format!("{context} init failed: {e}")))?;
    if let Some(src) = override_val
        && !src.is_null()
    {
        let src_obj = src
            .as_object()
            .ok_or_else(|| to_py_err(format!("{context} override must be a mapping/object")))?;
        if let Some(dst) = base.as_object_mut() {
            for (k, v) in src_obj {
                let mapped = aliases
                    .iter()
                    .find(|(alias, _)| alias == &k.as_str())
                    .map(|(_, canon)| *canon)
                    .unwrap_or(k.as_str());
                dst.insert(mapped.to_string(), v.clone());
            }
        }
    }
    serde_json::from_value(base).map_err(|e| to_py_err(format!("Invalid {context}: {e}")))
}

/// Helper verifying [`FromPyObjectOwned`] usability with `PySerde<T>`.
#[allow(dead_code)]
fn _assert_owned_extract<'py, T: DeserializeOwned>()
where
    PySerde<T>: FromPyObjectOwned<'py>,
{
}

#[cfg(test)]
mod tests {
    use super::*;
    use satya_schema::EconomicTier;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct SamplePayload {
        flag: bool,
        count: i32,
        items: Vec<String>,
    }

    #[test]
    fn py_bridge_preserves_bool_vs_int_and_supports_serde_and_coded_enum() {
        Python::initialize();
        Python::attach(|py| {
            let payload = SamplePayload {
                flag: true,
                count: 1,
                items: vec!["alpha".to_string(), "beta".to_string()],
            };
            let py_obj = PySerde(payload.clone()).into_pyobject(py).unwrap();
            let roundtrip: PySerde<SamplePayload> = py_obj.extract().unwrap();
            assert_eq!(roundtrip.0, payload);

            // JSON string fallback still works identically for legacy callers
            let json_str = PyString::new(py, r#"{"flag": false, "count": 42, "items": ["z"]}"#);
            let from_json: SamplePayload =
                from_py_object(&json_str.into_any(), "SamplePayload").unwrap();
            assert!(!from_json.flag);
            assert_eq!(from_json.count, 42);

            // CodedEnum extraction & helpful error listing T::ALL
            let tier: PyEnum<EconomicTier> = PyString::new(py, "low-income").extract().unwrap();
            assert_eq!(tier.0, EconomicTier::LowIncome);
            let err = parse_coded_enum::<EconomicTier>("invalid_tier", "economic_tier")
                .unwrap_err()
                .to_string();
            assert!(err.contains("MIDDLE_CLASS") && err.contains("LOW_INCOME"));
        });
    }
}
