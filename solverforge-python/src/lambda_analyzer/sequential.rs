//! Sequential expression pattern extraction.
//!
//! Handles patterns like:
//! ```python
//! var1 = expr1
//! var2 = func(var1)
//! return method(var2)
//! ```

use pyo3::prelude::*;
use pyo3::types::PyList;
use solverforge_core::wasm::Expression;
use std::collections::HashMap;

use super::ast_convert::{infer_expression_type, InferredType};
use super::conditionals;
use super::type_inference::infer_expression_class;

/// Function type for AST to Expression conversion.
pub type ConvertFn =
    fn(Python<'_>, &Bound<'_, PyAny>, &[String], &str) -> PyResult<Option<Expression>>;

/// Function type for building method call expressions.
pub type BuildMethodCallFn =
    fn(Python<'_>, Expression, &str, &[Expression], &str) -> PyResult<Expression>;

/// Try to extract expression from sequential variable assignments.
///
/// Matches patterns like:
/// ```python
/// var1 = expr1
/// var2 = func(var1)
/// return method(var2)
/// ```
///
/// This is converted by substituting local variables with their expressions.
pub fn try_extract_sequential_expression_pattern(
    py: Python<'_>,
    stmts: &[Bound<'_, PyAny>],
    arg_names: &[String],
    class_hint: &str,
    convert_fn: ConvertFn,
    build_method_call_fn: BuildMethodCallFn,
) -> PyResult<Expression> {
    // Collect variable assignments: var_name -> AST expression
    let mut local_vars: HashMap<String, Bound<'_, PyAny>> = HashMap::new();
    let mut return_node: Option<Bound<'_, PyAny>> = None;
    let mut early_return_if: Option<(Expression, Expression)> = None;

    for (idx, stmt) in stmts.iter().enumerate() {
        let stmt_type = stmt.get_type().name()?.to_string();

        match stmt_type.as_str() {
            "If" => {
                // Handle early return pattern at the start
                if idx == 0 {
                    if let Some((cond, ret_val)) = conditionals::detect_early_return(
                        py, stmt, arg_names, class_hint, convert_fn,
                    )? {
                        early_return_if = Some((cond, ret_val));
                    }
                }
            }
            "Assign" => {
                let targets = stmt.getattr("targets")?;
                let targets_list = targets.cast::<PyList>()?;
                if targets_list.len() == 1 {
                    let target = targets_list.get_item(0)?;
                    let target_type = target.get_type().name()?.to_string();
                    if target_type == "Name" {
                        let var_name: String = target.getattr("id")?.extract()?;
                        let value = stmt.getattr("value")?;
                        local_vars.insert(var_name, value);
                    }
                }
            }
            "Return" => {
                let value = stmt.getattr("value")?;
                if !value.is_none() {
                    return_node = Some(value);
                }
            }
            _ => {}
        }
    }

    // Must have a return statement
    let return_ast = return_node.ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>("No return statement found")
    })?;

    log::debug!(
        "try_extract_sequential_expression_pattern: {} local vars, return_ast type: {}",
        local_vars.len(),
        return_ast
            .get_type()
            .name()
            .map(|n| n.to_string())
            .unwrap_or_default()
    );

    // Convert return expression with local variable substitution
    let result = convert_ast_with_local_var_substitution(
        py,
        &return_ast,
        arg_names,
        class_hint,
        &local_vars,
        convert_fn,
        build_method_call_fn,
    )?;

    // Wrap with early return if present
    if let Some((condition, early_value)) = early_return_if {
        // Check if branches produce i64 values (datetime, timedelta)
        let then_type = infer_expression_type(&early_value);
        let else_type = infer_expression_type(&result);
        let use_i64 = then_type == InferredType::I64 || else_type == InferredType::I64;

        return if use_i64 {
            Ok(Expression::IfThenElse64 {
                condition: Box::new(condition),
                then_branch: Box::new(early_value),
                else_branch: Box::new(result),
            })
        } else {
            Ok(Expression::IfThenElse {
                condition: Box::new(condition),
                then_branch: Box::new(early_value),
                else_branch: Box::new(result),
            })
        };
    }

    Ok(result)
}

/// Convert AST to expression, substituting local variable references.
fn convert_ast_with_local_var_substitution(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    arg_names: &[String],
    class_hint: &str,
    local_vars: &HashMap<String, Bound<'_, PyAny>>,
    convert_fn: ConvertFn,
    build_method_call_fn: BuildMethodCallFn,
) -> PyResult<Expression> {
    let node_type = node.get_type().name()?.to_string();

    // Check if this is a local variable reference
    if node_type == "Name" {
        let var_name: String = node.getattr("id")?.extract()?;

        // If it's a local variable, substitute with its expression
        if let Some(var_expr_ast) = local_vars.get(&var_name) {
            return convert_ast_with_local_var_substitution(
                py,
                var_expr_ast,
                arg_names,
                class_hint,
                local_vars,
                convert_fn,
                build_method_call_fn,
            );
        }
    }

    // Handle method calls specially - need to convert args with substitution
    if node_type == "Call" {
        let func = node.getattr("func")?;
        let func_type = func.get_type().name()?.to_string();

        if func_type == "Attribute" {
            let value = func.getattr("value")?;
            let method_name: String = func.getattr("attr")?.extract()?;

            // Convert the base object with substitution
            let base_expr = convert_ast_with_local_var_substitution(
                py,
                &value,
                arg_names,
                class_hint,
                local_vars,
                convert_fn,
                build_method_call_fn,
            )?;

            // Convert arguments with substitution
            let call_args = node.getattr("args")?;
            let args_list = call_args.cast::<PyList>()?;
            let mut converted_args = Vec::new();
            for arg in args_list.iter() {
                let arg_expr = convert_ast_with_local_var_substitution(
                    py,
                    &arg,
                    arg_names,
                    class_hint,
                    local_vars,
                    convert_fn,
                    build_method_call_fn,
                )?;
                converted_args.push(arg_expr);
            }

            // Infer base class and try to inline the method
            let base_class = infer_expression_class(py, &base_expr, class_hint)?
                .unwrap_or_else(|| class_hint.to_string());

            return build_method_call_fn(py, base_expr, &method_name, &converted_args, &base_class);
        }
    }

    // Handle binary operations with substitution
    if node_type == "BinOp" {
        let left = node.getattr("left")?;
        let right = node.getattr("right")?;
        let op = node.getattr("op")?;
        let op_type = op.get_type().name()?.to_string();

        let left_expr = convert_ast_with_local_var_substitution(
            py,
            &left,
            arg_names,
            class_hint,
            local_vars,
            convert_fn,
            build_method_call_fn,
        )?;
        let right_expr = convert_ast_with_local_var_substitution(
            py,
            &right,
            arg_names,
            class_hint,
            local_vars,
            convert_fn,
            build_method_call_fn,
        )?;

        let left_type = infer_expression_type(&left_expr);
        let right_type = infer_expression_type(&right_expr);
        let use_float = left_type == InferredType::F64 || right_type == InferredType::F64;
        let use_i64 = left_type == InferredType::I64 || right_type == InferredType::I64;
        return Ok(match op_type.as_str() {
            "Add" if use_float => Expression::FloatAdd {
                left: Box::new(left_expr),
                right: Box::new(right_expr),
            },
            "Add" if use_i64 => Expression::Add64 {
                left: Box::new(left_expr),
                right: Box::new(right_expr),
            },
            "Add" => Expression::Add {
                left: Box::new(left_expr),
                right: Box::new(right_expr),
            },
            "Sub" if use_float => Expression::FloatSub {
                left: Box::new(left_expr),
                right: Box::new(right_expr),
            },
            "Sub" if use_i64 => Expression::Sub64 {
                left: Box::new(left_expr),
                right: Box::new(right_expr),
            },
            "Sub" => Expression::Sub {
                left: Box::new(left_expr),
                right: Box::new(right_expr),
            },
            "Mult" if use_float => Expression::FloatMul {
                left: Box::new(left_expr),
                right: Box::new(right_expr),
            },
            "Mult" if use_i64 => Expression::Mul64 {
                left: Box::new(left_expr),
                right: Box::new(right_expr),
            },
            "Mult" => Expression::Mul {
                left: Box::new(left_expr),
                right: Box::new(right_expr),
            },
            "Div" | "TrueDiv" => Expression::FloatDiv {
                left: Box::new(left_expr),
                right: Box::new(right_expr),
            },
            "FloorDiv" if use_i64 => Expression::Div64 {
                left: Box::new(left_expr),
                right: Box::new(right_expr),
            },
            "FloorDiv" => Expression::Div {
                left: Box::new(left_expr),
                right: Box::new(right_expr),
            },
            _ => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Unsupported binary operator: {}",
                    op_type
                )));
            }
        });
    }

    // Handle subscript (tuple indexing) with substitution
    if node_type == "Subscript" {
        let value = node.getattr("value")?;
        let slice = node.getattr("slice")?;

        // Check if value is a local variable (tuple)
        let value_type = value.get_type().name()?.to_string();
        if value_type == "Name" {
            let var_name: String = value.getattr("id")?.extract()?;
            if let Some(var_expr_ast) = local_vars.get(&var_name) {
                // This is indexing into a local variable (likely a tuple)
                // Check if the variable was assigned from a tuple expression
                let var_expr_type = var_expr_ast.get_type().name()?.to_string();

                if var_expr_type == "Tuple" {
                    // Get the index
                    let slice_type = slice.get_type().name()?.to_string();
                    if slice_type == "Constant" {
                        if let Ok(index) = slice.getattr("value").and_then(|v| v.extract::<i64>()) {
                            // Get the indexed element from the tuple
                            let elts = var_expr_ast.getattr("elts")?;
                            let elts_list = elts.cast::<PyList>()?;
                            if (index as usize) < elts_list.len() {
                                let element = elts_list.get_item(index as usize)?;
                                return convert_ast_with_local_var_substitution(
                                    py,
                                    &element,
                                    arg_names,
                                    class_hint,
                                    local_vars,
                                    convert_fn,
                                    build_method_call_fn,
                                );
                            }
                        }
                    }
                }

                // If tuple var was assigned from a method call, we need to track which element
                // This is complex - for now, fall back to error
                if var_expr_type == "Call" {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                        "Tuple indexing from method call result not yet supported",
                    ));
                }
            }
        }
    }

    // Fall back to standard conversion
    convert_fn(py, node, arg_names, class_hint)?
        .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("Cannot convert expression"))
}
