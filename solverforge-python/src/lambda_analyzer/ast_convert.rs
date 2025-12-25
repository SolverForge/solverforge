//! AST to Expression conversion helpers.
//!
//! This module contains pure helper functions for converting Python AST nodes
//! to solverforge Expression trees. These are stateless functions that don't
//! require method inlining or registry access.

use pyo3::prelude::*;
use pyo3::types::PyList;
use solverforge_core::wasm::Expression;

/// Extract argument names from Python AST arguments node.
pub(crate) fn extract_arg_names(_py: Python<'_>, args: &Bound<'_, PyAny>) -> PyResult<Vec<String>> {
    let arg_list = args.getattr("args")?;
    let list = arg_list.cast::<PyList>()?;

    let mut names = Vec::new();
    for arg in list.iter() {
        let arg_name: String = arg.getattr("arg")?.extract()?;
        names.push(arg_name);
    }

    Ok(names)
}

/// Convert Python Compare AST node to Expression.
pub(crate) fn convert_compare_to_expression(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    arg_names: &[String],
    class_hint: &str,
    convert_fn: impl Fn(Python<'_>, &Bound<'_, PyAny>, &[String], &str) -> PyResult<Option<Expression>>,
) -> PyResult<Option<Expression>> {
    let left = node.getattr("left")?;
    let ops_list = node.getattr("ops")?.cast::<PyList>()?.clone();
    let comparators_list = node.getattr("comparators")?.cast::<PyList>()?.clone();

    let ops: Vec<Bound<'_, PyAny>> = ops_list.iter().collect();
    let comparators: Vec<Bound<'_, PyAny>> = comparators_list.iter().collect();

    if ops.len() != 1 || comparators.len() != 1 {
        // Multiple comparisons (a < b < c) not directly supported
        return Ok(None);
    }

    let left_expr = convert_fn(py, &left, arg_names, class_hint)?;
    let right_expr = convert_fn(py, &comparators[0], arg_names, class_hint)?;

    match (left_expr, right_expr) {
        (Some(left), Some(right)) => {
            let op_type = ops[0].get_type().name()?.to_string();

            let expr = match op_type.as_str() {
                "Eq" => Expression::Eq {
                    left: Box::new(left),
                    right: Box::new(right),
                },
                "NotEq" => Expression::Ne {
                    left: Box::new(left),
                    right: Box::new(right),
                },
                "Lt" => Expression::Lt {
                    left: Box::new(left),
                    right: Box::new(right),
                },
                "LtE" => Expression::Le {
                    left: Box::new(left),
                    right: Box::new(right),
                },
                "Gt" => Expression::Gt {
                    left: Box::new(left),
                    right: Box::new(right),
                },
                "GtE" => Expression::Ge {
                    left: Box::new(left),
                    right: Box::new(right),
                },
                "Is" => {
                    // Check for "is None" pattern
                    if matches!(right, Expression::Null) {
                        Expression::IsNull {
                            operand: Box::new(left),
                        }
                    } else {
                        Expression::Eq {
                            left: Box::new(left),
                            right: Box::new(right),
                        }
                    }
                }
                "IsNot" => {
                    // Check for "is not None" pattern
                    if matches!(right, Expression::Null) {
                        Expression::IsNotNull {
                            operand: Box::new(left),
                        }
                    } else {
                        Expression::Ne {
                            left: Box::new(left),
                            right: Box::new(right),
                        }
                    }
                }
                _ => return Ok(None),
            };

            Ok(Some(expr))
        }
        _ => Ok(None),
    }
}

/// Convert Python BoolOp AST node (and/or) to Expression.
pub(crate) fn convert_boolop_to_expression(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    arg_names: &[String],
    class_hint: &str,
    convert_fn: impl Fn(Python<'_>, &Bound<'_, PyAny>, &[String], &str) -> PyResult<Option<Expression>>,
) -> PyResult<Option<Expression>> {
    let op = node.getattr("op")?;
    let values_list = node.getattr("values")?.cast::<PyList>()?.clone();
    let values: Vec<Bound<'_, PyAny>> = values_list.iter().collect();

    if values.len() < 2 {
        return Ok(None);
    }

    let op_type = op.get_type().name()?.to_string();

    // Convert all operands
    let mut exprs: Vec<Expression> = Vec::new();
    for val in values.iter() {
        if let Some(expr) = convert_fn(py, val, arg_names, class_hint)? {
            exprs.push(expr);
        } else {
            return Ok(None);
        }
    }

    // Chain the operations
    let mut result = exprs.remove(0);
    for expr in exprs {
        result = match op_type.as_str() {
            "And" => Expression::And {
                left: Box::new(result),
                right: Box::new(expr),
            },
            "Or" => Expression::Or {
                left: Box::new(result),
                right: Box::new(expr),
            },
            _ => return Ok(None),
        };
    }

    Ok(Some(result))
}

/// Convert Python UnaryOp AST node to Expression.
pub(crate) fn convert_unaryop_to_expression(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    arg_names: &[String],
    class_hint: &str,
    convert_fn: impl Fn(Python<'_>, &Bound<'_, PyAny>, &[String], &str) -> PyResult<Option<Expression>>,
) -> PyResult<Option<Expression>> {
    let op = node.getattr("op")?;
    let operand = node.getattr("operand")?;

    let op_type = op.get_type().name()?.to_string();

    if let Some(operand_expr) = convert_fn(py, &operand, arg_names, class_hint)? {
        let expr = match op_type.as_str() {
            "Not" => Expression::Not {
                operand: Box::new(operand_expr),
            },
            "USub" => {
                // Unary minus: -x
                Expression::Sub {
                    left: Box::new(Expression::IntLiteral { value: 0 }),
                    right: Box::new(operand_expr),
                }
            }
            _ => return Ok(None),
        };
        Ok(Some(expr))
    } else {
        Ok(None)
    }
}

/// Convert Python BinOp AST node to Expression.
pub(crate) fn convert_binop_to_expression(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    arg_names: &[String],
    class_hint: &str,
    convert_fn: impl Fn(Python<'_>, &Bound<'_, PyAny>, &[String], &str) -> PyResult<Option<Expression>>,
) -> PyResult<Option<Expression>> {
    let op = node.getattr("op")?;
    let left = node.getattr("left")?;
    let right = node.getattr("right")?;

    let left_expr = convert_fn(py, &left, arg_names, class_hint)?;
    let right_expr = convert_fn(py, &right, arg_names, class_hint)?;

    match (left_expr, right_expr) {
        (Some(l), Some(r)) => {
            let op_type = op.get_type().name()?.to_string();

            let expr = match op_type.as_str() {
                "Add" => Expression::Add {
                    left: Box::new(l),
                    right: Box::new(r),
                },
                "Sub" => Expression::Sub {
                    left: Box::new(l),
                    right: Box::new(r),
                },
                "Mult" => Expression::Mul {
                    left: Box::new(l),
                    right: Box::new(r),
                },
                "Div" | "FloorDiv" => Expression::Div {
                    left: Box::new(l),
                    right: Box::new(r),
                },
                _ => return Ok(None),
            };

            Ok(Some(expr))
        }
        _ => Ok(None),
    }
}

/// Convert Python constant to Expression.
pub(crate) fn convert_constant_to_expression(
    node: &Bound<'_, PyAny>,
) -> PyResult<Option<Expression>> {
    let node_type = node.get_type().name()?.to_string();

    match node_type.as_str() {
        "Constant" => {
            // Python 3.13+ uses Constant for all literals
            let value = node.getattr("value")?;

            if value.is_none() {
                Ok(Some(Expression::Null))
            } else if let Ok(b) = value.extract::<bool>() {
                Ok(Some(Expression::BoolLiteral { value: b }))
            } else if let Ok(i) = value.extract::<i64>() {
                Ok(Some(Expression::IntLiteral { value: i }))
            } else if let Ok(f) = value.extract::<f64>() {
                Ok(Some(Expression::FloatLiteral { value: f }))
            } else if let Ok(s) = value.extract::<String>() {
                Ok(Some(Expression::StringLiteral { value: s }))
            } else {
                Ok(None)
            }
        }
        // Note: Num, Str, NameConstant are Python <3.8 legacy - not supported
        _ => Ok(None),
    }
}

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
