//! Guard condition detection for AST analysis.
//!
//! This module contains functions for detecting common guard patterns
//! in Python AST nodes, such as empty collection checks and null checks.

use pyo3::prelude::*;
use pyo3::types::PyList;

/// Check if a condition AST node is an "empty collection guard" pattern: len(x) == 0
pub(crate) fn is_empty_collection_guard(
    _py: Python<'_>,
    condition: &Bound<'_, PyAny>,
) -> PyResult<bool> {
    let node_type = condition.get_type().name()?.to_string();
    if node_type != "Compare" {
        return Ok(false);
    }

    // Check left side is len(...)
    let left = condition.getattr("left")?;
    let left_type = left.get_type().name()?.to_string();
    if left_type != "Call" {
        return Ok(false);
    }

    let func = left.getattr("func")?;
    let func_type = func.get_type().name()?.to_string();
    if func_type != "Name" {
        return Ok(false);
    }

    let func_name: String = func.getattr("id")?.extract()?;
    if func_name != "len" {
        return Ok(false);
    }

    // Check comparator is == 0
    let ops = condition.getattr("ops")?;
    let ops_list = ops.cast::<PyList>()?;
    if ops_list.len() != 1 {
        return Ok(false);
    }

    let op = ops_list.get_item(0)?;
    let op_type = op.get_type().name()?.to_string();
    if op_type != "Eq" {
        return Ok(false);
    }

    let comparators = condition.getattr("comparators")?;
    let comps_list = comparators.cast::<PyList>()?;
    if comps_list.len() != 1 {
        return Ok(false);
    }

    let comp = comps_list.get_item(0)?;
    let comp_type = comp.get_type().name()?.to_string();
    if comp_type != "Constant" {
        return Ok(false);
    }

    if let Ok(value) = comp.getattr("value") {
        if let Ok(0i64) = value.extract() {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Check if a condition AST node is an "is not None" check: X is not None
pub(crate) fn is_not_none_check(_py: Python<'_>, condition: &Bound<'_, PyAny>) -> PyResult<bool> {
    let node_type = condition.get_type().name()?.to_string();
    if node_type != "Compare" {
        return Ok(false);
    }

    // Check operator is IsNot
    let ops = condition.getattr("ops")?;
    let ops_list = ops.cast::<PyList>()?;
    if ops_list.len() != 1 {
        return Ok(false);
    }

    let op = ops_list.get_item(0)?;
    let op_type = op.get_type().name()?.to_string();
    if op_type != "IsNot" {
        return Ok(false);
    }

    // Check comparator is None
    let comparators = condition.getattr("comparators")?;
    let comps_list = comparators.cast::<PyList>()?;
    if comps_list.len() != 1 {
        return Ok(false);
    }

    let comp = comps_list.get_item(0)?;
    let comp_type = comp.get_type().name()?.to_string();

    // Check if it's the None constant
    if comp_type == "Constant" {
        if let Ok(value) = comp.getattr("value") {
            if value.is_none() {
                return Ok(true);
            }
        }
    } else if comp_type == "Name" {
        let name: String = comp.getattr("id")?.extract()?;
        if name == "None" {
            return Ok(true);
        }
    }

    Ok(false)
}
