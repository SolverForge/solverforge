//! Conditional pattern extraction for lambda analysis.
//!
//! This module handles extraction of if/else patterns from Python AST,
//! including early-return patterns and assignment-based conditionals.

use pyo3::prelude::*;
use pyo3::types::PyList;
use solverforge_core::wasm::Expression;

use super::ast_convert::{is_empty_collection_guard, is_not_none_check};

/// Type alias for AST conversion functions.
pub type ConvertFn =
    fn(Python<'_>, &Bound<'_, PyAny>, &[String], &str) -> PyResult<Option<Expression>>;

/// Type alias for accumulation pattern extraction functions.
pub type AccumFn = fn(Python<'_>, &Bound<'_, PyList>, &[String], &str) -> PyResult<Expression>;

/// Extract if-then-else expression from an If statement.
///
/// Handles patterns like:
/// ```python
/// if condition:
///     return expr1
/// else:
///     return expr2
/// ```
pub(super) fn extract_if_else(
    py: Python<'_>,
    if_stmt: &Bound<'_, PyAny>,
    arg_names: &[String],
    class_hint: &str,
    convert_fn: ConvertFn,
) -> PyResult<Expression> {
    // Extract condition
    let condition_node = if_stmt.getattr("test")?;
    let condition = convert_fn(py, &condition_node, arg_names, class_hint)?.ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>("Cannot convert if condition")
    })?;

    // Extract then branch (body)
    let body = if_stmt.getattr("body")?;
    let body_list = body.cast::<PyList>()?;
    let mut then_expr = None;
    for stmt in body_list.iter() {
        let stmt_type = stmt.get_type().name()?.to_string();
        if stmt_type == "Return" {
            let value = stmt.getattr("value")?;
            if !value.is_none() {
                then_expr = convert_fn(py, &value, arg_names, class_hint)?;
                break;
            } else {
                then_expr = Some(Expression::Null);
                break;
            }
        }
    }

    let then_expr = then_expr.ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>("If statement body must contain return")
    })?;

    // Extract else branch (orelse)
    let orelse = if_stmt.getattr("orelse")?;
    let orelse_list = orelse.cast::<PyList>()?;
    let mut else_expr = None;

    for stmt in orelse_list.iter() {
        let stmt_type = stmt.get_type().name()?.to_string();
        if stmt_type == "Return" {
            let value = stmt.getattr("value")?;
            if !value.is_none() {
                else_expr = convert_fn(py, &value, arg_names, class_hint)?;
                break;
            } else {
                else_expr = Some(Expression::Null);
                break;
            }
        } else if stmt_type == "If" {
            // Nested if - recursively extract
            else_expr = Some(extract_if_else(
                py, &stmt, arg_names, class_hint, convert_fn,
            )?);
            break;
        }
    }

    let else_expr = else_expr.ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "If statement must have else branch with return",
        )
    })?;

    Ok(Expression::IfThenElse {
        condition: Box::new(condition),
        then_branch: Box::new(then_expr),
        else_branch: Box::new(else_expr),
    })
}

/// Extract assignment-based pattern for methods that assign to self.field instead of returning.
///
/// Handles patterns like:
/// ```python
/// if condition1:
///     self.field = expr1
/// elif condition2:
///     self.field = expr2
/// else:
///     self.field = expr3
/// ```
pub(super) fn extract_assignment_if(
    py: Python<'_>,
    stmts: &[Bound<'_, PyAny>],
    arg_names: &[String],
    class_hint: &str,
    convert_fn: ConvertFn,
) -> PyResult<Expression> {
    // Look for an If statement that contains assignments
    for stmt in stmts {
        let stmt_type = stmt.get_type().name()?.to_string();
        if stmt_type == "If" {
            if let Ok(expr) = extract_if_assignment(py, stmt, arg_names, class_hint, convert_fn) {
                return Ok(expr);
            }
        }
    }
    Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
        "No assignment pattern found",
    ))
}

/// Extract expression from if/elif/else that assigns to self.field.
fn extract_if_assignment(
    py: Python<'_>,
    if_stmt: &Bound<'_, PyAny>,
    arg_names: &[String],
    class_hint: &str,
    convert_fn: ConvertFn,
) -> PyResult<Expression> {
    // Extract condition
    let condition_node = if_stmt.getattr("test")?;
    let condition = convert_fn(py, &condition_node, arg_names, class_hint)?.ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>("Cannot convert if condition")
    })?;

    // Extract then branch - look for assignment or return
    let body = if_stmt.getattr("body")?;
    let body_list = body.cast::<PyList>()?;
    let then_expr = extract_branch_value(py, body_list, arg_names, class_hint, convert_fn)?;

    // Extract else branch
    let orelse = if_stmt.getattr("orelse")?;
    let orelse_list = orelse.cast::<PyList>()?;

    let else_expr = if orelse_list.is_empty() {
        // No else branch - this shouldn't happen for complete assignment patterns
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Assignment pattern requires else branch",
        ));
    } else {
        // Check if it's an elif (nested If) or else block
        let first_stmt = orelse_list.get_item(0)?;
        let first_type = first_stmt.get_type().name()?.to_string();
        if first_type == "If" {
            // Recursively handle elif
            extract_if_assignment(py, &first_stmt, arg_names, class_hint, convert_fn)?
        } else {
            // Extract from else block
            extract_branch_value(py, orelse_list, arg_names, class_hint, convert_fn)?
        }
    };

    Ok(Expression::IfThenElse {
        condition: Box::new(condition),
        then_branch: Box::new(then_expr),
        else_branch: Box::new(else_expr),
    })
}

/// Extract the value expression from a branch (handles both Return and Assign to self.field).
fn extract_branch_value(
    py: Python<'_>,
    stmts: &Bound<'_, PyList>,
    arg_names: &[String],
    class_hint: &str,
    convert_fn: ConvertFn,
) -> PyResult<Expression> {
    for stmt in stmts.iter() {
        let stmt_type = stmt.get_type().name()?.to_string();

        if stmt_type == "Return" {
            let value = stmt.getattr("value")?;
            if value.is_none() {
                return Ok(Expression::Null);
            }
            return convert_fn(py, &value, arg_names, class_hint)?.ok_or_else(|| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>("Cannot convert return value")
            });
        }

        if stmt_type == "Assign" {
            // Check if it's assigning to self.field
            let targets = stmt.getattr("targets")?;
            let targets_list = targets.cast::<PyList>()?;
            if !targets_list.is_empty() {
                let target = targets_list.get_item(0)?;
                let target_type = target.get_type().name()?.to_string();
                if target_type == "Attribute" {
                    let target_value = target.getattr("value")?;
                    let target_value_type = target_value.get_type().name()?.to_string();
                    if target_value_type == "Name" {
                        let name: String = target_value.getattr("id")?.extract()?;
                        if name == "self" || arg_names.first() == Some(&name) {
                            // This is self.field = expr, extract the value
                            let value = stmt.getattr("value")?;
                            if value.is_none() {
                                return Ok(Expression::Null);
                            }
                            return convert_fn(py, &value, arg_names, class_hint)?.ok_or_else(
                                || {
                                    PyErr::new::<pyo3::exceptions::PyValueError, _>(
                                        "Cannot convert assigned value",
                                    )
                                },
                            );
                        }
                    }
                }
            }
        }

        if stmt_type == "Expr" {
            // Expression statement - might be a None assignment represented differently
            let value = stmt.getattr("value")?;
            let value_type = value.get_type().name()?.to_string();
            if value_type == "Constant" {
                if let Ok(is_none) = value.is_none().then_some(true).ok_or(false) {
                    if is_none {
                        return Ok(Expression::Null);
                    }
                }
            }
        }
    }

    Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
        "Branch must contain return or assignment to self.field",
    ))
}

/// Extract if-early-return pattern where if body returns and remaining statements are the else.
///
/// Handles patterns like:
/// ```python
/// if condition:
///     return x
/// return y  # This is the implicit else
/// ```
pub(super) fn extract_early_return(
    py: Python<'_>,
    if_stmt: &Bound<'_, PyAny>,
    remaining_stmts: &[Bound<'_, PyAny>],
    arg_names: &[String],
    class_hint: &str,
    convert_fn: ConvertFn,
    try_accum_fn: AccumFn,
) -> PyResult<Expression> {
    // Extract condition
    let condition_node = if_stmt.getattr("test")?;
    let condition_opt = convert_fn(py, &condition_node, arg_names, class_hint)?;

    // Extract then branch from if body
    let body = if_stmt.getattr("body")?;
    let body_list = body.cast::<PyList>()?;
    let mut then_expr = None;
    for stmt in body_list.iter() {
        let stmt_type = stmt.get_type().name()?.to_string();
        if stmt_type == "Return" {
            let value = stmt.getattr("value")?;
            if !value.is_none() {
                then_expr = convert_fn(py, &value, arg_names, class_hint)?;
                break;
            } else {
                then_expr = Some(Expression::Null);
                break;
            }
        }
    }

    let then_expr = then_expr.ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "If body must contain return for early-return pattern",
        )
    })?;

    // Check for patterns where condition can't be converted but we can use a fallback
    log::debug!(
        "extract_early_return: condition_opt={:?}, then_expr={:?}",
        condition_opt,
        then_expr
    );

    // Pattern 1: Empty collection guard - works even if condition can be converted
    let is_guard = is_empty_collection_guard(py, &condition_node)?;
    let is_zero = matches!(then_expr, Expression::IntLiteral { value: 0 });
    log::debug!(
        "Empty guard check: is_guard={}, is_zero={}, then_expr={:?}",
        is_guard,
        is_zero,
        then_expr
    );

    if is_guard && is_zero {
        // Try accumulation pattern - Sum handles empty collections naturally
        let remaining_list = PyList::new(py, remaining_stmts)?;
        log::debug!(
            "Trying accumulation pattern on {} remaining statements",
            remaining_stmts.len()
        );
        match try_accum_fn(py, &remaining_list, arg_names, class_hint) {
            Ok(accum_expr) => {
                log::debug!("Accumulation pattern succeeded: {:?}", accum_expr);
                return Ok(accum_expr);
            }
            Err(e) => {
                log::debug!("Accumulation pattern failed: {}", e);
            }
        }
    }

    if condition_opt.is_none() {
        // Pattern 2: "if X is not None: ...; return fallback" - use the fallback
        if is_not_none_check(py, &condition_node)? {
            // The remaining statements should have a fallback return
            for stmt in remaining_stmts.iter() {
                let stmt_type = stmt.get_type().name()?.to_string();
                if stmt_type == "Return" {
                    let value = stmt.getattr("value")?;
                    if !value.is_none() {
                        if let Some(fallback_expr) = convert_fn(py, &value, arg_names, class_hint)?
                        {
                            log::debug!("Using fallback for 'is not None' pattern");
                            return Ok(fallback_expr);
                        }
                    }
                }
            }
        }

        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Cannot convert if condition",
        ));
    }

    let condition = condition_opt.unwrap();

    // Extract else from remaining statements - find a return or nested if-early-return
    let mut else_expr = None;
    for (i, stmt) in remaining_stmts.iter().enumerate() {
        let stmt_type = stmt.get_type().name()?.to_string();
        if stmt_type == "Return" {
            let value = stmt.getattr("value")?;
            if !value.is_none() {
                else_expr = convert_fn(py, &value, arg_names, class_hint)?;
                break;
            }
        } else if stmt_type == "If" {
            // Nested if-early-return in else branch
            let remaining_after = &remaining_stmts[i + 1..];
            else_expr = Some(extract_early_return(
                py,
                stmt,
                remaining_after,
                arg_names,
                class_hint,
                convert_fn,
                try_accum_fn,
            )?);
            break;
        }
    }

    // If no direct return found, try accumulation pattern on remaining statements
    if else_expr.is_none() {
        let remaining_list = PyList::new(py, remaining_stmts)?;
        if let Ok(accum_expr) = try_accum_fn(py, &remaining_list, arg_names, class_hint) {
            else_expr = Some(accum_expr);
        }
    }

    let else_expr = else_expr.ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Remaining statements must contain return or accumulation pattern",
        )
    })?;

    Ok(Expression::IfThenElse {
        condition: Box::new(condition),
        then_branch: Box::new(then_expr),
        else_branch: Box::new(else_expr),
    })
}

/// Try to detect and extract an if-early-return pattern from a single If statement.
///
/// Matches pattern: `if condition: return value`
/// Returns (condition_expr, early_return_value) on success.
pub(super) fn detect_early_return(
    py: Python<'_>,
    if_stmt: &Bound<'_, PyAny>,
    arg_names: &[String],
    class_hint: &str,
    convert_fn: ConvertFn,
) -> PyResult<Option<(Expression, Expression)>> {
    let test = if_stmt.getattr("test")?;
    let body = if_stmt.getattr("body")?;
    let body_list = body.cast::<PyList>()?;
    let orelse = if_stmt.getattr("orelse")?;
    let orelse_list = orelse.cast::<PyList>()?;

    // Must have exactly one statement in body and empty else
    if body_list.len() != 1 || !orelse_list.is_empty() {
        return Ok(None);
    }

    let body_stmt = body_list.get_item(0)?;
    let stmt_type = body_stmt.get_type().name()?.to_string();

    // Body must be a Return statement
    if stmt_type != "Return" {
        return Ok(None);
    }

    let ret_value = body_stmt.getattr("value")?;
    if ret_value.is_none() {
        return Ok(None);
    }

    // Convert condition and return value to expressions
    let condition_expr = convert_fn(py, &test, arg_names, class_hint)?;
    let return_expr = convert_fn(py, &ret_value, arg_names, class_hint)?;

    if let (Some(cond), Some(ret)) = (condition_expr, return_expr) {
        Ok(Some((cond, ret)))
    } else {
        Ok(None)
    }
}
