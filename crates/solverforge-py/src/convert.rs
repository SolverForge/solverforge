//! Conversion utilities between Python and Rust types.

use std::sync::Arc;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyList;

use solverforge_core::score::HardSoftScore;
use solverforge_dynamic::{DynamicValue, FieldType};

/// Convert a Python value to a DynamicValue.
pub fn py_to_dynamic(value: &Bound<'_, PyAny>) -> PyResult<DynamicValue> {
    if value.is_none() {
        Ok(DynamicValue::None)
    } else if let Ok(v) = value.extract::<i64>() {
        Ok(DynamicValue::I64(v))
    } else if let Ok(v) = value.extract::<f64>() {
        Ok(DynamicValue::F64(v))
    } else if let Ok(v) = value.extract::<bool>() {
        Ok(DynamicValue::Bool(v))
    } else if let Ok(v) = value.extract::<String>() {
        Ok(DynamicValue::String(Arc::from(v)))
    } else if let Ok(list) = value.downcast::<PyList>() {
        let items: PyResult<Vec<_>> = list.iter().map(|item| py_to_dynamic(&item)).collect();
        Ok(DynamicValue::List(items?))
    } else {
        Err(PyValueError::new_err(format!(
            "Cannot convert Python value to DynamicValue: {:?}",
            value
        )))
    }
}

/// Convert a DynamicValue to a Python value.
pub fn dynamic_to_py(py: Python<'_>, value: &DynamicValue) -> Py<PyAny> {
    match value {
        DynamicValue::None => py.None(),
        DynamicValue::I64(v) => v.into_pyobject(py).unwrap().into_any().unbind(),
        DynamicValue::F64(v) => v.into_pyobject(py).unwrap().into_any().unbind(),
        DynamicValue::Bool(v) => v.into_pyobject(py).unwrap().to_owned().into_any().unbind(),
        DynamicValue::String(v) => v.as_ref().into_pyobject(py).unwrap().into_any().unbind(),
        DynamicValue::Ref(c, e) => (*c, *e).into_pyobject(py).unwrap().into_any().unbind(),
        DynamicValue::FactRef(c, e) => (*c, *e).into_pyobject(py).unwrap().into_any().unbind(),
        DynamicValue::DateTime(ts) => ts.into_pyobject(py).unwrap().into_any().unbind(),
        DynamicValue::Date(days) => days.into_pyobject(py).unwrap().into_any().unbind(),
        DynamicValue::List(items) | DynamicValue::Set(items) => {
            let py_items: Vec<_> = items.iter().map(|i| dynamic_to_py(py, i)).collect();
            PyList::new(py, py_items).unwrap().into_any().unbind()
        }
    }
}

/// Parse a field type from a string.
pub fn parse_field_type(type_str: &str) -> PyResult<FieldType> {
    match type_str.to_lowercase().as_str() {
        "int" | "i64" | "integer" => Ok(FieldType::I64),
        "float" | "f64" | "double" => Ok(FieldType::F64),
        "str" | "string" => Ok(FieldType::String),
        "bool" | "boolean" => Ok(FieldType::Bool),
        "ref" | "reference" => Ok(FieldType::Ref),
        "list" => Ok(FieldType::List),
        _ => Err(PyValueError::new_err(format!(
            "Unknown field type: {}",
            type_str
        ))),
    }
}

/// Parse a score weight from a string like "1hard" or "1hard/2soft".
pub fn parse_weight(weight_str: &str) -> PyResult<HardSoftScore> {
    let weight_str = weight_str.trim();

    // Handle simple formats: "1hard", "2soft"
    if weight_str.ends_with("hard") {
        let num_str = weight_str.trim_end_matches("hard");
        let num: i64 = num_str.parse().map_err(|e| {
            PyValueError::new_err(format!("Invalid hard weight '{}': {}", num_str, e))
        })?;
        return Ok(HardSoftScore::of_hard(num));
    }

    if weight_str.ends_with("soft") && !weight_str.contains('/') {
        let num_str = weight_str.trim_end_matches("soft");
        let num: i64 = num_str.parse().map_err(|e| {
            PyValueError::new_err(format!("Invalid soft weight '{}': {}", num_str, e))
        })?;
        return Ok(HardSoftScore::of_soft(num));
    }

    // Handle full format: "1hard/2soft"
    let parts: Vec<&str> = weight_str.split('/').collect();
    if parts.len() == 2 {
        let hard_str = parts[0].trim().trim_end_matches("hard");
        let soft_str = parts[1].trim().trim_end_matches("soft");

        let hard: i64 = hard_str.parse().map_err(|e| {
            PyValueError::new_err(format!("Invalid hard weight '{}': {}", hard_str, e))
        })?;
        let soft: i64 = soft_str.parse().map_err(|e| {
            PyValueError::new_err(format!("Invalid soft weight '{}': {}", soft_str, e))
        })?;

        return Ok(HardSoftScore::of(hard, soft));
    }

    Err(PyValueError::new_err(format!(
        "Invalid weight format '{}'. Expected formats: '1hard', '2soft', or '1hard/2soft'",
        weight_str
    )))
}
