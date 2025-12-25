//! Loop and accumulation pattern extraction for lambda analysis.
//!
//! This module handles extraction of loop patterns from Python AST,
//! including sum accumulation and mutable variable tracking.

use pyo3::prelude::*;
use pyo3::types::PyList;
use solverforge_core::wasm::Expression;

use super::conditionals::{self, ConvertFn};
use super::patterns::{LoopContext, MutableLoopVar};
use super::registry::CLASS_REGISTRY;
use super::type_inference::{
    create_sum_over_collection, get_field_type_and_register, infer_item_type,
};

/// Type alias for method call builder function.
pub type BuildMethodCallFn =
    fn(Python<'_>, Expression, &str, &[Expression], &str) -> PyResult<Expression>;

/// Detect and extract accumulation patterns from method body.
///
/// Recognizes patterns like:
/// ```python
/// def method(self):
///     if len(self.collection) == 0:  # Optional early return
///         return 0
///     total = 0
///     for item in self.collection:
///         total += item.field
///     total += final_value  # Optional post-loop addition
///     return total
/// ```
pub(super) fn try_extract_accumulation_pattern(
    py: Python<'_>,
    body_list: &Bound<'_, PyList>,
    arg_names: &[String],
    class_hint: &str,
    convert_fn: ConvertFn,
    build_method_call_fn: BuildMethodCallFn,
) -> PyResult<Expression> {
    // Pattern: [If-early-return?, Assign*, For, AugAssign*, Return]

    let mut accumulator_var: Option<String> = None;
    let mut pre_loop_assigns: Vec<(String, Bound<'_, PyAny>)> = Vec::new();
    let mut for_loop_node: Option<Bound<'_, PyAny>> = None;
    let mut post_loop_augassigns: Vec<Bound<'_, PyAny>> = Vec::new();
    let mut return_var: Option<String> = None;
    let mut found_for = false;
    let mut early_return_if: Option<(Expression, Expression)> = None; // (condition, early_value)

    for (idx, stmt) in body_list.iter().enumerate() {
        let stmt_type = stmt.get_type().name()?.to_string();

        match stmt_type.as_str() {
            "If" => {
                // Check for early return pattern at the start: if condition: return value
                if idx == 0 && !found_for {
                    if let Some((cond, early_val)) = conditionals::detect_early_return(
                        py, &stmt, arg_names, class_hint, convert_fn,
                    )? {
                        early_return_if = Some((cond, early_val));
                    }
                }
            }
            "Assign" => {
                if !found_for {
                    // Pre-loop assignment
                    let targets = stmt.getattr("targets")?;
                    let targets_list = targets.cast::<PyList>()?;
                    if targets_list.len() == 1 {
                        let target = targets_list.get_item(0)?;
                        let target_type = target.get_type().name()?.to_string();

                        if target_type == "Name" {
                            let var_name: String = target.getattr("id")?.extract()?;
                            let value = stmt.getattr("value")?;
                            let value_type = value.get_type().name()?.to_string();

                            // Check if value is 0 (accumulator init)
                            if value_type == "Constant" {
                                let const_value = value.getattr("value").ok();
                                if let Some(v) = const_value {
                                    if let Ok(0i64) = v.extract::<i64>() {
                                        accumulator_var = Some(var_name.clone());
                                    }
                                }
                            }

                            // Also track as pre-loop assign (for mutable var detection)
                            pre_loop_assigns.push((var_name, value));
                        }
                    }
                }
            }
            "For" => {
                for_loop_node = Some(stmt.clone());
                found_for = true;
            }
            "AugAssign" => {
                if found_for {
                    // Post-loop augmented assignment
                    post_loop_augassigns.push(stmt.clone());
                }
            }
            "Return" => {
                let ret_value = stmt.getattr("value")?;
                if !ret_value.is_none() {
                    let ret_type = ret_value.get_type().name()?.to_string();
                    if ret_type == "Name" {
                        return_var = Some(ret_value.getattr("id")?.extract()?);
                    }
                }
            }
            _ => {}
        }
    }

    log::debug!(
        "try_extract_accumulation_pattern: accumulator_var={:?}, for_loop_node={}, return_var={:?}, pre_loop_assigns={}",
        accumulator_var,
        for_loop_node.is_some(),
        return_var,
        pre_loop_assigns.len()
    );

    // Check if we found the pattern
    if let (Some(accum_var), Some(for_loop), Some(ret_var)) =
        (accumulator_var, for_loop_node, return_var)
    {
        if accum_var == ret_var {
            // Extract the sum from the for loop (returns LoopContext)
            let loop_ctx = extract_sum_from_for_loop(
                py,
                &for_loop,
                &accum_var,
                arg_names,
                class_hint,
                &pre_loop_assigns,
                convert_fn,
                build_method_call_fn,
            )?;

            let mut result = loop_ctx.sum_expr;

            // Add post-loop augmented assignments
            for augassign in &post_loop_augassigns {
                if let Some(post_expr) = try_extract_post_loop_augassign(
                    py,
                    augassign,
                    &accum_var,
                    arg_names,
                    class_hint,
                    &loop_ctx.collection_expr,
                    &loop_ctx.item_class_name,
                    &loop_ctx.mutable_vars,
                    convert_fn,
                    build_method_call_fn,
                )? {
                    result = Expression::Add {
                        left: Box::new(result),
                        right: Box::new(post_expr),
                    };
                }
            }

            // Wrap in IfThenElse if there's an early return
            if let Some((condition, early_value)) = early_return_if {
                result = Expression::IfThenElse {
                    condition: Box::new(condition),
                    then_branch: Box::new(early_value),
                    else_branch: Box::new(result),
                };
            }

            return Ok(result);
        }
    }

    Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
        "Not an accumulation pattern",
    ))
}

/// Try to extract expression from a post-loop AugAssign.
///
/// Matches pattern: `accum_var += expr`
///
/// For mutable variables like `previous_location`, substitutes with
/// `LastElement(collection).field` to represent the final value.
#[allow(clippy::too_many_arguments)]
fn try_extract_post_loop_augassign(
    py: Python<'_>,
    augassign: &Bound<'_, PyAny>,
    accum_var: &str,
    arg_names: &[String],
    class_hint: &str,
    collection_expr: &Expression,
    item_class_name: &str,
    mutable_vars: &[MutableLoopVar],
    convert_fn: ConvertFn,
    build_method_call_fn: BuildMethodCallFn,
) -> PyResult<Option<Expression>> {
    let target = augassign.getattr("target")?;
    let target_type = target.get_type().name()?.to_string();

    if target_type != "Name" {
        return Ok(None);
    }

    let target_var: String = target.getattr("id")?.extract()?;
    if target_var != accum_var {
        return Ok(None);
    }

    let op = augassign.getattr("op")?;
    let op_type = op.get_type().name()?.to_string();
    if op_type != "Add" {
        return Ok(None);
    }

    let value = augassign.getattr("value")?;

    // Convert with mutable var substitution using loop context
    convert_ast_with_mutable_var_substitution(
        py,
        &value,
        arg_names,
        class_hint,
        collection_expr,
        item_class_name,
        mutable_vars,
        convert_fn,
        build_method_call_fn,
    )
}

/// Convert AST to expression, substituting mutable loop variables with their final values.
///
/// For post-loop expressions, mutable vars like `previous_location` should refer to
/// the last element's field value: `LastElement(collection).field`
#[allow(clippy::too_many_arguments)]
fn convert_ast_with_mutable_var_substitution(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    arg_names: &[String],
    class_hint: &str,
    collection_expr: &Expression,
    item_class_name: &str,
    mutable_vars: &[MutableLoopVar],
    convert_fn: ConvertFn,
    build_method_call_fn: BuildMethodCallFn,
) -> PyResult<Option<Expression>> {
    let node_type = node.get_type().name()?.to_string();

    // Check if this is a method call on a mutable variable
    if node_type == "Call" {
        let func = node.getattr("func")?;
        let func_type = func.get_type().name()?.to_string();

        if func_type == "Attribute" {
            let value = func.getattr("value")?;
            let value_type = value.get_type().name()?.to_string();
            let method_name: String = func.getattr("attr")?.extract()?;

            if value_type == "Name" {
                let base_name: String = value.getattr("id")?.extract()?;

                // Check if this is a mutable variable
                for mv in mutable_vars {
                    if base_name == mv.name {
                        // This is a call like: mutable_var.method(args)
                        // Substitute mutable_var with LastElement(collection).field
                        let last_elem = Expression::LastElement {
                            collection: Box::new(collection_expr.clone()),
                            item_class_name: item_class_name.to_string(),
                        };

                        // Access the tracked field on the last element
                        let last_elem_field = Expression::FieldAccess {
                            object: Box::new(last_elem),
                            class_name: item_class_name.to_string(),
                            field_name: mv.item_field.clone(),
                        };

                        // Convert call arguments
                        let call_args = node.getattr("args")?;
                        let args_list = call_args.cast::<PyList>()?;
                        let mut converted_args = Vec::new();
                        for arg in args_list.iter() {
                            if let Some(arg_expr) = convert_fn(py, &arg, arg_names, class_hint)? {
                                converted_args.push(arg_expr);
                            } else {
                                return Ok(None);
                            }
                        }

                        // Infer the class of the field (e.g., Location for visit.location)
                        let field_class =
                            get_field_type_and_register(py, item_class_name, &mv.item_field)?
                                .unwrap_or_else(|| class_hint.to_string());

                        // Try to inline the method
                        return build_method_call_fn(
                            py,
                            last_elem_field,
                            &method_name,
                            &converted_args,
                            &field_class,
                        )
                        .map(Some);
                    }
                }
            }
        }
    }

    // Fall back to standard conversion
    convert_fn(py, node, arg_names, class_hint)
}

/// Extract a sum expression from a for loop that accumulates values.
///
/// Converts: for item in collection: acc += item.field
/// To: Sum of item.field for each item in collection
///
/// Also handles "previous element" patterns where a variable tracks
/// the previous iteration's value.
///
/// Returns LoopContext with the sum expression and info needed for post-loop processing.
#[allow(clippy::too_many_arguments)]
fn extract_sum_from_for_loop(
    py: Python<'_>,
    for_loop: &Bound<'_, PyAny>,
    accum_var: &str,
    arg_names: &[String],
    class_hint: &str,
    pre_loop_assigns: &[(String, Bound<'_, PyAny>)],
    convert_fn: ConvertFn,
    build_method_call_fn: BuildMethodCallFn,
) -> PyResult<LoopContext> {
    // Extract loop variable and iterable
    let target = for_loop.getattr("target")?;
    let target_type = target.get_type().name()?.to_string();

    if target_type != "Name" {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Complex loop targets not supported",
        ));
    }

    let loop_var: String = target.getattr("id")?.extract()?;
    let iter_expr = for_loop.getattr("iter")?;

    // The iterable should be something like self.collection
    if let Some(collection_expr) = convert_fn(py, &iter_expr, arg_names, class_hint)? {
        // Infer the item type from the collection
        let item_class_hint = infer_item_type(py, &collection_expr)?;

        // Now extract what's being accumulated from the loop body
        let body = for_loop.getattr("body")?;
        let body_list = body.cast::<PyList>()?;

        // Detect mutable loop variables (assigned before loop, updated in loop)
        let mutable_vars = detect_mutable_loop_vars(py, body_list, &loop_var, pre_loop_assigns)?;

        // Look for the accumulation statement: accum_var += something
        for stmt in body_list.iter() {
            let stmt_type = stmt.get_type().name()?.to_string();

            if stmt_type == "AugAssign" {
                let target = stmt.getattr("target")?;
                let target_type = target.get_type().name()?.to_string();

                if target_type == "Name" {
                    let target_var: String = target.getattr("id")?.extract()?;

                    if target_var == accum_var {
                        let op = stmt.getattr("op")?;
                        let op_type = op.get_type().name()?.to_string();

                        if op_type == "Add" {
                            let value = stmt.getattr("value")?;

                            // Create arg list with loop variable
                            let mut loop_arg_names = arg_names.to_vec();
                            loop_arg_names.push(loop_var.clone());

                            // Also add mutable var names so they're recognized
                            for mv in &mutable_vars {
                                loop_arg_names.push(mv.name.clone());
                            }

                            // Try to convert the accumulated expression
                            let accumulated_expr = convert_ast_with_mutable_vars(
                                py,
                                &value,
                                &loop_arg_names,
                                &item_class_hint,
                                &loop_var,
                                &mutable_vars,
                                arg_names,
                                class_hint,
                                convert_fn,
                                build_method_call_fn,
                            )?;

                            if let Some(expr) = accumulated_expr {
                                let loop_var_param_index = (arg_names.len()) as u32;

                                let sum_expr = create_sum_over_collection(
                                    expr,
                                    &loop_var,
                                    collection_expr.clone(),
                                    loop_var_param_index,
                                    &item_class_hint,
                                );

                                return Ok(LoopContext {
                                    sum_expr,
                                    collection_expr,
                                    item_class_name: item_class_hint.clone(),
                                    mutable_vars,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
        "Could not extract sum pattern from for loop",
    ))
}

/// Detect variables that are assigned before the loop and updated inside the loop.
/// These track "previous element" values.
fn detect_mutable_loop_vars(
    _py: Python<'_>,
    body_list: &Bound<'_, PyList>,
    loop_var: &str,
    pre_loop_assigns: &[(String, Bound<'_, PyAny>)],
) -> PyResult<Vec<MutableLoopVar>> {
    let mut result = Vec::new();

    // Find assignments in loop body that update a pre-loop variable
    for stmt in body_list.iter() {
        let stmt_type = stmt.get_type().name()?.to_string();

        if stmt_type == "Assign" {
            let targets = stmt.getattr("targets")?;
            let targets_list = targets.cast::<PyList>()?;
            if targets_list.len() == 1 {
                let target = targets_list.get_item(0)?;
                let target_type = target.get_type().name()?.to_string();

                if target_type == "Name" {
                    let var_name: String = target.getattr("id")?.extract()?;

                    // Check if this variable was assigned before the loop
                    for (pre_var, pre_value) in pre_loop_assigns {
                        if &var_name == pre_var {
                            // Check if the new value is loop_var.field
                            let value = stmt.getattr("value")?;
                            let value_type = value.get_type().name()?.to_string();

                            if value_type == "Attribute" {
                                let attr_value = value.getattr("value")?;
                                let attr_type = attr_value.get_type().name()?.to_string();

                                if attr_type == "Name" {
                                    let base_name: String = attr_value.getattr("id")?.extract()?;
                                    if base_name == loop_var {
                                        let field_name: String =
                                            value.getattr("attr")?.extract()?;

                                        // Clone pre_value into 'static lifetime by leaking
                                        // This is safe as we're in a short-lived analysis context
                                        let init_expr: Bound<'static, PyAny> =
                                            unsafe { std::mem::transmute(pre_value.clone()) };

                                        result.push(MutableLoopVar {
                                            name: var_name.clone(),
                                            init_expr,
                                            item_field: field_name,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(result)
}

/// Convert AST to expression, handling mutable loop variables.
///
/// When a mutable variable is referenced, it's replaced with:
/// - For first iteration (item.previous_X is None): the init expression
/// - For other iterations: item.previous_X.field
#[allow(clippy::too_many_arguments)]
fn convert_ast_with_mutable_vars(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    arg_names: &[String],
    class_hint: &str,
    loop_var: &str,
    mutable_vars: &[MutableLoopVar],
    outer_arg_names: &[String],
    outer_class_hint: &str,
    convert_fn: ConvertFn,
    build_method_call_fn: BuildMethodCallFn,
) -> PyResult<Option<Expression>> {
    let node_type = node.get_type().name()?.to_string();

    // Check if this is a method call on a mutable variable
    if node_type == "Call" {
        let func = node.getattr("func")?;
        let func_type = func.get_type().name()?.to_string();

        if func_type == "Attribute" {
            let value = func.getattr("value")?;
            let value_type = value.get_type().name()?.to_string();
            let method_name: String = func.getattr("attr")?.extract()?;

            if value_type == "Name" {
                let base_name: String = value.getattr("id")?.extract()?;

                // Check if this is a mutable variable
                for mv in mutable_vars {
                    if base_name == mv.name {
                        // This is a call like: mutable_var.method(args)
                        // Replace with: if loop_var.previous_X is None: init.method(args) else: loop_var.previous_X.field.method(args)

                        let call_args = node.getattr("args")?;
                        let args_list = call_args.cast::<PyList>()?;

                        // Convert call arguments
                        let mut converted_args = Vec::new();
                        for arg in args_list.iter() {
                            if let Some(arg_expr) = convert_fn(py, &arg, arg_names, class_hint)? {
                                converted_args.push(arg_expr);
                            } else {
                                return Ok(None);
                            }
                        }

                        // Find the "previous_X" shadow variable name
                        // Convention: if tracking "location" from Visit, look for "previous_visit"
                        let previous_shadow = find_previous_shadow_variable(py, class_hint)?;

                        if let Some(prev_field) = previous_shadow {
                            // Build the expression:
                            // if loop_var.prev_field is None:
                            //     init_expr.method(args)
                            // else:
                            //     loop_var.prev_field.item_field.method(args)

                            let loop_var_idx = arg_names
                                .iter()
                                .position(|n| n == loop_var)
                                .ok_or_else(|| {
                                    PyErr::new::<pyo3::exceptions::PyValueError, _>(
                                        "Loop var not found",
                                    )
                                })?;
                            let loop_var_param = Expression::Param {
                                index: loop_var_idx as u32,
                            };

                            // Condition: loop_var.prev_field is None
                            let condition = Expression::IsNull {
                                operand: Box::new(Expression::FieldAccess {
                                    object: Box::new(loop_var_param.clone()),
                                    class_name: class_hint.to_string(),
                                    field_name: prev_field.clone(),
                                }),
                            };

                            // Then branch: init_expr.method(args)
                            let init_base =
                                convert_fn(py, &mv.init_expr, outer_arg_names, outer_class_hint)?
                                    .ok_or_else(|| {
                                    PyErr::new::<pyo3::exceptions::PyValueError, _>(
                                        "Cannot convert init expr",
                                    )
                                })?;

                            let then_branch = build_method_call_fn(
                                py,
                                init_base,
                                &method_name,
                                &converted_args,
                                outer_class_hint,
                            )?;

                            // Else branch: loop_var.prev_field.item_field.method(args)
                            // First get the previous item
                            let prev_item = Expression::FieldAccess {
                                object: Box::new(loop_var_param.clone()),
                                class_name: class_hint.to_string(),
                                field_name: prev_field.clone(),
                            };
                            // Then get the field from it
                            let prev_item_field = Expression::FieldAccess {
                                object: Box::new(prev_item),
                                class_name: class_hint.to_string(),
                                field_name: mv.item_field.clone(),
                            };

                            let else_branch = build_method_call_fn(
                                py,
                                prev_item_field,
                                &method_name,
                                &converted_args,
                                outer_class_hint,
                            )?;

                            return Ok(Some(Expression::IfThenElse {
                                condition: Box::new(condition),
                                then_branch: Box::new(then_branch),
                                else_branch: Box::new(else_branch),
                            }));
                        }
                    }
                }
            }
        }
    }

    // Default: use standard conversion
    convert_fn(py, node, arg_names, class_hint)
}

/// Find the "previous" shadow variable for a class.
/// Returns the field name like "previous_visit" for Visit class.
pub(super) fn find_previous_shadow_variable(
    py: Python<'_>,
    class_name: &str,
) -> PyResult<Option<String>> {
    // Clone class reference while holding lock, then release before Python operations
    let class_ref: Option<Py<PyAny>> = {
        let registry = CLASS_REGISTRY.read().unwrap();
        if let Some(ref map) = *registry {
            map.get(class_name).map(|c| c.clone_ref(py))
        } else {
            None
        }
    };

    if let Some(class) = class_ref {
        // Look for annotations containing "previous" in the name
        if let Ok(annotations) = class.bind(py).getattr("__annotations__") {
            if let Ok(dict) = annotations.cast::<pyo3::types::PyDict>() {
                for (key, _value) in dict.iter() {
                    if let Ok(key_str) = key.extract::<String>() {
                        if key_str.starts_with("previous_") {
                            return Ok(Some(key_str));
                        }
                    }
                }
            }
        }
    }

    Ok(None)
}
