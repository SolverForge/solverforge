//! AST to Expression conversion.
//!
//! This module contains functions for converting Python AST nodes
//! to solverforge Expression trees.

use pyo3::prelude::*;
use pyo3::types::PyList;
use solverforge_core::wasm::Expression;

use super::constants::get_class_constant;
use super::registry::{get_method_from_class, CLASS_REGISTRY};
use super::type_inference::infer_expression_class;

/// Check if an expression produces a float result.
/// Used to select between int/float arithmetic operations.
pub fn is_float_expr(expr: &Expression) -> bool {
    match expr {
        Expression::FloatLiteral { .. } => true,
        Expression::FloatAdd { .. }
        | Expression::FloatSub { .. }
        | Expression::FloatMul { .. }
        | Expression::FloatDiv { .. } => true,
        Expression::Sqrt { .. }
        | Expression::FloatAbs { .. }
        | Expression::Sin { .. }
        | Expression::Cos { .. }
        | Expression::Asin { .. }
        | Expression::Acos { .. }
        | Expression::Atan { .. }
        | Expression::Atan2 { .. }
        | Expression::Radians { .. } => true,
        Expression::IntToFloat { .. } => true,
        Expression::IfThenElse {
            then_branch,
            else_branch,
            ..
        } => is_float_expr(then_branch) || is_float_expr(else_branch),
        _ => false,
    }
}

/// Check if an expression produces an i64 result.
/// Used to select between i32/i64 comparison and arithmetic operations.
pub fn is_i64_expr(expr: &Expression) -> bool {
    match expr {
        // i64 arithmetic operations
        Expression::Add64 { .. }
        | Expression::Sub64 { .. }
        | Expression::Mul64 { .. }
        | Expression::Div64 { .. } => true,
        // Large integer literals that don't fit in i32
        Expression::IntLiteral { value } => *value < i32::MIN as i64 || *value > i32::MAX as i64,
        Expression::IfThenElse {
            then_branch,
            else_branch,
            ..
        } => is_i64_expr(then_branch) || is_i64_expr(else_branch),
        _ => false,
    }
}

/// Check if a FieldAccess expression references an i64 field.
/// This looks up the Python type annotation and checks for datetime types.
pub fn is_i64_field_access(py: Python<'_>, expr: &Expression) -> bool {
    if let Expression::FieldAccess {
        class_name,
        field_name,
        ..
    } = expr
    {
        // Look up the field type from the class registry
        let class_ref: Option<Py<PyAny>> = {
            let registry = CLASS_REGISTRY.read().unwrap();
            if let Some(ref map) = *registry {
                map.get(class_name).map(|c| c.clone_ref(py))
            } else {
                None
            }
        };

        if let Some(class) = class_ref {
            let class_bound = class.bind(py);
            if let Ok(get_type_hints) = py
                .import("typing")
                .and_then(|m| m.getattr("get_type_hints"))
            {
                if let Ok(hints) = get_type_hints.call1((&class_bound,)) {
                    if let Ok(field_type) = hints.get_item(field_name) {
                        // Check if field type is datetime
                        if let Ok(type_name) = field_type.getattr("__name__") {
                            if let Ok(name) = type_name.extract::<String>() {
                                // datetime fields are stored as i64 timestamps
                                return name == "datetime";
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

/// Check if either expression produces an i64 result, including field access.
pub fn is_i64_operand(py: Python<'_>, expr: &Expression) -> bool {
    is_i64_expr(expr) || is_i64_field_access(py, expr)
}

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
            let use_i64 = is_i64_operand(py, &left) || is_i64_operand(py, &right);

            let expr = match op_type.as_str() {
                "Eq" => {
                    if use_i64 {
                        Expression::Eq64 {
                            left: Box::new(left),
                            right: Box::new(right),
                        }
                    } else {
                        Expression::Eq {
                            left: Box::new(left),
                            right: Box::new(right),
                        }
                    }
                }
                "NotEq" => {
                    if use_i64 {
                        Expression::Ne64 {
                            left: Box::new(left),
                            right: Box::new(right),
                        }
                    } else {
                        Expression::Ne {
                            left: Box::new(left),
                            right: Box::new(right),
                        }
                    }
                }
                "Lt" => {
                    if use_i64 {
                        Expression::Lt64 {
                            left: Box::new(left),
                            right: Box::new(right),
                        }
                    } else {
                        Expression::Lt {
                            left: Box::new(left),
                            right: Box::new(right),
                        }
                    }
                }
                "LtE" => {
                    if use_i64 {
                        Expression::Le64 {
                            left: Box::new(left),
                            right: Box::new(right),
                        }
                    } else {
                        Expression::Le {
                            left: Box::new(left),
                            right: Box::new(right),
                        }
                    }
                }
                "Gt" => {
                    if use_i64 {
                        Expression::Gt64 {
                            left: Box::new(left),
                            right: Box::new(right),
                        }
                    } else {
                        Expression::Gt {
                            left: Box::new(left),
                            right: Box::new(right),
                        }
                    }
                }
                "GtE" => {
                    if use_i64 {
                        Expression::Ge64 {
                            left: Box::new(left),
                            right: Box::new(right),
                        }
                    } else {
                        Expression::Ge {
                            left: Box::new(left),
                            right: Box::new(right),
                        }
                    }
                }
                "Is" => {
                    // Check for "is None" pattern
                    if matches!(right, Expression::Null) {
                        Expression::IsNull {
                            operand: Box::new(left),
                        }
                    } else if use_i64 {
                        Expression::Eq64 {
                            left: Box::new(left),
                            right: Box::new(right),
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
                    } else if use_i64 {
                        Expression::Ne64 {
                            left: Box::new(left),
                            right: Box::new(right),
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
                "Add" => {
                    if is_float_expr(&l) || is_float_expr(&r) {
                        Expression::FloatAdd {
                            left: Box::new(l),
                            right: Box::new(r),
                        }
                    } else {
                        Expression::Add {
                            left: Box::new(l),
                            right: Box::new(r),
                        }
                    }
                }
                "Sub" => {
                    if is_float_expr(&l) || is_float_expr(&r) {
                        Expression::FloatSub {
                            left: Box::new(l),
                            right: Box::new(r),
                        }
                    } else {
                        Expression::Sub {
                            left: Box::new(l),
                            right: Box::new(r),
                        }
                    }
                }
                "Mult" => {
                    if is_float_expr(&l) || is_float_expr(&r) {
                        Expression::FloatMul {
                            left: Box::new(l),
                            right: Box::new(r),
                        }
                    } else {
                        Expression::Mul {
                            left: Box::new(l),
                            right: Box::new(r),
                        }
                    }
                }
                // Python `/` is always true division (returns float)
                "Div" => Expression::FloatDiv {
                    left: Box::new(l),
                    right: Box::new(r),
                },
                "FloorDiv" => Expression::Div {
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

/// Convert Python AST node to Expression tree.
///
/// This is the main AST conversion function that handles all node types.
pub(crate) fn convert_ast_to_expression(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    arg_names: &[String],
    class_hint: &str,
) -> PyResult<Option<Expression>> {
    let node_type = node.get_type().name()?.to_string();
    let class_name = class_hint.to_string();

    match node_type.as_str() {
        "Attribute" => {
            // Field access: x.field or class constant: x.CONSTANT
            let value = node.getattr("value")?;
            let attr: String = node.getattr("attr")?.extract()?;

            if let Some(base_expr) = convert_ast_to_expression(py, &value, arg_names, class_hint)? {
                // Check if this is a class constant (self.CONST or param.CONST)
                // Only check when base is a direct parameter reference
                if matches!(base_expr, Expression::Param { .. }) {
                    if let Some(constant_expr) = get_class_constant(py, class_hint, &attr)? {
                        log::debug!("Inlined class constant {}.{} in lambda", class_hint, attr);
                        return Ok(Some(constant_expr));
                    }
                }

                Ok(Some(Expression::FieldAccess {
                    object: Box::new(base_expr),
                    class_name,
                    field_name: attr,
                }))
            } else {
                Ok(None)
            }
        }

        "Name" => {
            // Variable reference
            let id: String = node.getattr("id")?.extract()?;

            // Check if it's a lambda parameter
            if let Some(idx) = arg_names.iter().position(|n| n == &id) {
                Ok(Some(Expression::Param { index: idx as u32 }))
            } else if id == "None" {
                Ok(Some(Expression::Null))
            } else if id == "True" {
                Ok(Some(Expression::BoolLiteral { value: true }))
            } else if id == "False" {
                Ok(Some(Expression::BoolLiteral { value: false }))
            } else {
                // External reference - not supported
                Ok(None)
            }
        }

        "Compare" => {
            // Comparison: x < y, x == y, x is None, etc.
            convert_compare_to_expression(
                py,
                node,
                arg_names,
                class_hint,
                convert_ast_to_expression,
            )
        }

        "BoolOp" => {
            // Boolean operation: and, or
            convert_boolop_to_expression(py, node, arg_names, class_hint, convert_ast_to_expression)
        }

        "UnaryOp" => {
            // Unary operation: not
            convert_unaryop_to_expression(
                py,
                node,
                arg_names,
                class_hint,
                convert_ast_to_expression,
            )
        }

        "BinOp" => {
            // Binary operation: +, -, *, /
            convert_binop_to_expression(py, node, arg_names, class_hint, convert_ast_to_expression)
        }

        "Constant" => {
            // Literal value
            convert_constant_to_expression(node)
        }

        "Call" => {
            // Method call: obj.method() or function() or module.function()
            convert_call_to_expression(py, node, arg_names, class_hint)
        }

        _ => Ok(None),
    }
}

/// Convert a Call AST node to Expression.
fn convert_call_to_expression(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    arg_names: &[String],
    class_hint: &str,
) -> PyResult<Option<Expression>> {
    let func = node.getattr("func")?;
    let func_type = func.get_type().name()?.to_string();

    if func_type == "Attribute" {
        let value = func.getattr("value")?;
        let method_name: String = func.getattr("attr")?.extract()?;
        let value_type = value.get_type().name()?.to_string();

        // Check if this is a module-level call like math.sin()
        if value_type == "Name" {
            let module_name: String = value.getattr("id")?.extract()?;
            if module_name == "math" {
                return convert_math_call(py, node, &method_name, arg_names, class_hint);
            }
        }

        // Method call: obj.method()
        return convert_method_call(py, node, &value, &method_name, arg_names, class_hint);
    } else if func_type == "Name" {
        // Built-in function call like max(), min(), timedelta(), etc.
        let func_name: String = func.getattr("id")?.extract()?;
        return convert_builtin_call(py, node, &func_name, arg_names, class_hint);
    }

    // Other types of calls - not supported for inlining
    Ok(None)
}

/// Convert math module function call.
fn convert_math_call(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    method_name: &str,
    arg_names: &[String],
    class_hint: &str,
) -> PyResult<Option<Expression>> {
    let args_node = node.getattr("args")?;
    let args_list = args_node.cast::<PyList>()?;

    let mut call_args = Vec::new();
    for arg in args_list.iter() {
        if let Some(arg_expr) = convert_ast_to_expression(py, &arg, arg_names, class_hint)? {
            call_args.push(arg_expr);
        } else {
            return Ok(None);
        }
    }

    match method_name {
        "sin" if call_args.len() == 1 => Ok(Some(Expression::Sin {
            operand: Box::new(call_args[0].clone()),
        })),
        "cos" if call_args.len() == 1 => Ok(Some(Expression::Cos {
            operand: Box::new(call_args[0].clone()),
        })),
        "sqrt" if call_args.len() == 1 => Ok(Some(Expression::Sqrt {
            operand: Box::new(call_args[0].clone()),
        })),
        "asin" if call_args.len() == 1 => Ok(Some(Expression::Asin {
            operand: Box::new(call_args[0].clone()),
        })),
        "acos" if call_args.len() == 1 => Ok(Some(Expression::Acos {
            operand: Box::new(call_args[0].clone()),
        })),
        "atan" if call_args.len() == 1 => Ok(Some(Expression::Atan {
            operand: Box::new(call_args[0].clone()),
        })),
        "atan2" if call_args.len() == 2 => Ok(Some(Expression::Atan2 {
            y: Box::new(call_args[0].clone()),
            x: Box::new(call_args[1].clone()),
        })),
        "radians" if call_args.len() == 1 => Ok(Some(Expression::Radians {
            operand: Box::new(call_args[0].clone()),
        })),
        "floor" if call_args.len() == 1 => Ok(Some(Expression::Floor {
            operand: Box::new(call_args[0].clone()),
        })),
        "ceil" if call_args.len() == 1 => Ok(Some(Expression::Ceil {
            operand: Box::new(call_args[0].clone()),
        })),
        "fabs" if call_args.len() == 1 => Ok(Some(Expression::FloatAbs {
            operand: Box::new(call_args[0].clone()),
        })),
        _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Unsupported math function: math.{}()",
            method_name
        ))),
    }
}

/// Convert method call on an object.
fn convert_method_call(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    value: &Bound<'_, PyAny>,
    method_name: &str,
    arg_names: &[String],
    class_hint: &str,
) -> PyResult<Option<Expression>> {
    // Get the object expression
    let Some(obj_expr) = convert_ast_to_expression(py, value, arg_names, class_hint)? else {
        return Ok(None);
    };

    // Convert method call arguments
    let args_node = node.getattr("args")?;
    let args_list = args_node.cast::<PyList>()?;

    // Determine the actual class of the object for method lookup
    let obj_class = infer_expression_class(py, &obj_expr, class_hint)?
        .unwrap_or_else(|| class_hint.to_string());

    // Try to inline the method - look it up in the registry and analyze
    if let Some(method) = get_method_from_class(py, &obj_class, method_name) {
        match super::analyze_method_body(py, &method, &obj_class) {
            Ok(method_body) => {
                // Substitute parameters: obj_expr becomes Param(0), call_args become Param(1), etc.
                let mut inlined = method_body;

                // Substitute method parameters with call arguments
                // The object is Param(0) in the method, and obj_expr in the call
                inlined = super::substitute_param(inlined, 0, &obj_expr);

                // Substitute other parameters
                for (i, arg) in args_list.iter().enumerate() {
                    if let Some(arg_expr) =
                        convert_ast_to_expression(py, &arg, arg_names, class_hint)?
                    {
                        // In the method, args start at Param(1)
                        inlined = super::substitute_param(inlined, (i + 1) as u32, &arg_expr);
                    }
                }

                return Ok(Some(inlined));
            }
            Err(e) => {
                // Method couldn't be inlined - return error
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Cannot inline method {}.{}(): {}",
                    obj_class, method_name, e
                )));
            }
        }
    }

    // Method not found in registry - return error
    Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
        "Cannot inline method {}.{}() - class not registered. Register the class with register_class() first.",
        obj_class, method_name
    )))
}

/// Convert built-in function call.
fn convert_builtin_call(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    func_name: &str,
    arg_names: &[String],
    class_hint: &str,
) -> PyResult<Option<Expression>> {
    let args_node = node.getattr("args")?;
    let args_list = args_node.cast::<PyList>()?;

    // Convert positional arguments to expressions
    let mut call_args = Vec::new();
    for arg in args_list.iter() {
        if let Some(arg_expr) = convert_ast_to_expression(py, &arg, arg_names, class_hint)? {
            call_args.push(arg_expr);
        } else {
            return Ok(None);
        }
    }

    // Parse keyword arguments
    let keywords_node = node.getattr("keywords")?;
    let keywords_list = keywords_node.cast::<PyList>()?;
    let mut keyword_args: Vec<(String, i64)> = Vec::new();
    for kw in keywords_list.iter() {
        let arg_name_opt = kw.getattr("arg")?;
        if !arg_name_opt.is_none() {
            let arg_name: String = arg_name_opt.extract()?;
            let arg_value = kw.getattr("value")?;
            // Try to extract as integer constant
            let value_type = arg_value.get_type().name()?.to_string();
            if value_type == "Constant" {
                if let Ok(i) = arg_value.getattr("value")?.extract::<i64>() {
                    keyword_args.push((arg_name, i));
                }
            } else if value_type == "UnaryOp" {
                // Handle negative numbers like -1
                let op = arg_value.getattr("op")?;
                let op_type = op.get_type().name()?.to_string();
                if op_type == "USub" {
                    let operand = arg_value.getattr("operand")?;
                    if let Ok(i) = operand.getattr("value")?.extract::<i64>() {
                        keyword_args.push((arg_name, -i));
                    }
                }
            }
        }
    }

    // Handle specific built-in functions
    match func_name {
        "max" if call_args.len() == 2 => {
            // max(a, b) as a ternary: a > b ? a : b
            Ok(Some(Expression::IfThenElse {
                condition: Box::new(Expression::Gt {
                    left: Box::new(call_args[0].clone()),
                    right: Box::new(call_args[1].clone()),
                }),
                then_branch: Box::new(call_args[0].clone()),
                else_branch: Box::new(call_args[1].clone()),
            }))
        }
        "min" if call_args.len() == 2 => {
            // min(a, b) as a ternary: a < b ? a : b
            Ok(Some(Expression::IfThenElse {
                condition: Box::new(Expression::Lt {
                    left: Box::new(call_args[0].clone()),
                    right: Box::new(call_args[1].clone()),
                }),
                then_branch: Box::new(call_args[0].clone()),
                else_branch: Box::new(call_args[1].clone()),
            }))
        }
        "abs" if call_args.len() == 1 => {
            // abs(a) as: a < 0 ? -a : a
            Ok(Some(Expression::IfThenElse {
                condition: Box::new(Expression::Lt {
                    left: Box::new(call_args[0].clone()),
                    right: Box::new(Expression::IntLiteral { value: 0 }),
                }),
                then_branch: Box::new(Expression::Mul {
                    left: Box::new(Expression::IntLiteral { value: -1 }),
                    right: Box::new(call_args[0].clone()),
                }),
                else_branch: Box::new(call_args[0].clone()),
            }))
        }
        "len" if call_args.len() == 1 => {
            // len(collection) -> Length expression
            Ok(Some(Expression::Length {
                collection: Box::new(call_args[0].clone()),
            }))
        }
        "round" if call_args.len() == 1 => {
            // round(x) -> Round expression (WASM f64.nearest)
            Ok(Some(Expression::Round {
                operand: Box::new(call_args[0].clone()),
            }))
        }
        "int" if call_args.len() == 1 => {
            // int(x) -> FloatToInt for float values
            Ok(Some(Expression::FloatToInt {
                operand: Box::new(call_args[0].clone()),
            }))
        }
        "float" if call_args.len() == 1 => {
            // float(x) -> IntToFloat for int values
            Ok(Some(Expression::IntToFloat {
                operand: Box::new(call_args[0].clone()),
            }))
        }
        "timedelta" => {
            // Convert timedelta to integer seconds
            // Supports: days, hours, minutes, seconds keyword args
            let mut total_seconds: i64 = 0;
            for (name, value) in &keyword_args {
                match name.as_str() {
                    "days" => total_seconds += value * 86400,
                    "hours" => total_seconds += value * 3600,
                    "minutes" => total_seconds += value * 60,
                    "seconds" => total_seconds += value,
                    _ => {}
                }
            }
            Ok(Some(Expression::IntLiteral {
                value: total_seconds,
            }))
        }
        _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Cannot inline function call: {}()",
            func_name
        ))),
    }
}
