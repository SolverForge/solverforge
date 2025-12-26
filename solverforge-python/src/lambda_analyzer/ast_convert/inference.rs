//! AST type inference functions.
//!
//! This module contains functions for inferring types from Python AST nodes
//! without converting them to Expression trees. This enables single-pass
//! type-correct expression emission.

use pyo3::prelude::*;
use pyo3::types::PyList;
use solverforge_core::wasm::Expression;

use super::types::InferredType;
use crate::lambda_analyzer::registry::CLASS_REGISTRY;

/// Infer the type of an AST node WITHOUT converting it to an Expression.
///
/// This is the first pass of type inference - we analyze the AST structure
/// to determine what type each expression will produce, THEN we emit
/// the correct expressions in a single pass.
pub(crate) fn infer_ast_type(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    arg_names: &[String],
    class_hint: &str,
) -> PyResult<InferredType> {
    let node_type = node.get_type().name()?.to_string();

    match node_type.as_str() {
        "Constant" => {
            let value = node.getattr("value")?;
            if value.is_none() {
                Ok(InferredType::Null)
            } else if value.extract::<bool>().is_ok() {
                Ok(InferredType::Bool)
            } else if value.extract::<i64>().is_ok() {
                // Integer literal - default to I32 unless value exceeds range
                if let Ok(i) = value.extract::<i64>() {
                    if i < i32::MIN as i64 || i > i32::MAX as i64 {
                        Ok(InferredType::I64)
                    } else {
                        Ok(InferredType::I32)
                    }
                } else {
                    Ok(InferredType::I32)
                }
            } else if value.extract::<f64>().is_ok() {
                Ok(InferredType::F64)
            } else if value.extract::<String>().is_ok() {
                Ok(InferredType::String)
            } else {
                Ok(InferredType::Unknown)
            }
        }

        "Name" => {
            let id: String = node.getattr("id")?.extract()?;
            if id == "None" {
                Ok(InferredType::Null)
            } else if id == "True" || id == "False" {
                Ok(InferredType::Bool)
            } else if arg_names.contains(&id) {
                // Lambda parameter - type depends on class, treat as Unknown
                Ok(InferredType::Unknown)
            } else {
                Ok(InferredType::Unknown)
            }
        }

        "Attribute" => {
            // Field access - look up field type from class registry
            let attr: String = node.getattr("attr")?.extract()?;

            // Determine the class of the base object
            let value = node.getattr("value")?;
            let base_class = infer_base_class(py, &value, arg_names, class_hint)?;

            // Look up the field type
            infer_field_type(py, &base_class, &attr)
        }

        "BinOp" => {
            let op = node.getattr("op")?;
            let op_type = op.get_type().name()?.to_string();
            let left = node.getattr("left")?;
            let right = node.getattr("right")?;

            // Python `/` always produces float
            if op_type == "Div" {
                return Ok(InferredType::F64);
            }

            // For other ops, promote the operand types
            let left_type = infer_ast_type(py, &left, arg_names, class_hint)?;
            let right_type = infer_ast_type(py, &right, arg_names, class_hint)?;

            Ok(left_type.promote(right_type))
        }

        "Compare" => {
            // Comparisons always return boolean
            Ok(InferredType::Bool)
        }

        "BoolOp" => {
            // Boolean operations return boolean
            Ok(InferredType::Bool)
        }

        "UnaryOp" => {
            let op = node.getattr("op")?;
            let op_type = op.get_type().name()?.to_string();

            if op_type == "Not" {
                Ok(InferredType::Bool)
            } else {
                // USub, UAdd - propagate operand type
                let operand = node.getattr("operand")?;
                infer_ast_type(py, &operand, arg_names, class_hint)
            }
        }

        "Call" => {
            let func = node.getattr("func")?;
            let func_type = func.get_type().name()?.to_string();

            if func_type == "Name" {
                let func_name: String = func.getattr("id")?.extract()?;
                match func_name.as_str() {
                    // timedelta always produces i64 (seconds for datetime arithmetic)
                    "timedelta" => Ok(InferredType::I64),
                    // len returns i32
                    "len" => Ok(InferredType::I32),
                    // round/floor/ceil return i32 (truncated from float)
                    "round" => Ok(InferredType::I32),
                    // int() casts to i32
                    "int" => Ok(InferredType::I32),
                    // float() casts to f64
                    "float" => Ok(InferredType::F64),
                    // max/min/abs - propagate from arguments
                    "max" | "min" | "abs" => {
                        let args = node.getattr("args")?;
                        let args_list = args.cast::<PyList>()?;
                        if !args_list.is_empty() {
                            infer_ast_type(py, &args_list.get_item(0)?, arg_names, class_hint)
                        } else {
                            Ok(InferredType::Unknown)
                        }
                    }
                    _ => Ok(InferredType::Unknown),
                }
            } else if func_type == "Attribute" {
                // Method call or module.function
                let value = func.getattr("value")?;
                let method_name: String = func.getattr("attr")?.extract()?;
                let value_type = value.get_type().name()?.to_string();

                if value_type == "Name" {
                    let module_name: String = value.getattr("id")?.extract()?;
                    if module_name == "math" {
                        // All math functions return float
                        return Ok(InferredType::F64);
                    }
                }

                // Method call - would need to analyze the method body
                // For now, return Unknown and let the method inlining handle it
                let _ = method_name;
                Ok(InferredType::Unknown)
            } else {
                Ok(InferredType::Unknown)
            }
        }

        _ => Ok(InferredType::Unknown),
    }
}

/// Infer the class name of a base expression for field access.
pub(crate) fn infer_base_class(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    arg_names: &[String],
    class_hint: &str,
) -> PyResult<String> {
    let node_type = node.get_type().name()?.to_string();

    match node_type.as_str() {
        "Name" => {
            let id: String = node.getattr("id")?.extract()?;
            // If it's the first lambda parameter, use class_hint
            if arg_names.first() == Some(&id) {
                Ok(class_hint.to_string())
            } else {
                Ok(id)
            }
        }
        "Attribute" => {
            // Chained field access: x.field1.field2
            let attr: String = node.getattr("attr")?.extract()?;
            let value = node.getattr("value")?;
            let base_class = infer_base_class(py, &value, arg_names, class_hint)?;

            // Look up the type of this field to get the class for the next field
            if let Some(field_class) = get_field_class_name(py, &base_class, &attr) {
                Ok(field_class)
            } else {
                Ok(base_class)
            }
        }
        _ => Ok(class_hint.to_string()),
    }
}

/// Look up a field's class name from the registry.
fn get_field_class_name(py: Python<'_>, class_name: &str, field_name: &str) -> Option<String> {
    let class_ref: Option<Py<PyAny>> = {
        let registry = CLASS_REGISTRY.read().unwrap();
        if let Some(ref map) = *registry {
            map.get(class_name).map(|c| c.clone_ref(py))
        } else {
            None
        }
    };

    let class = class_ref?;
    let class_bound = class.bind(py);

    let get_type_hints = py
        .import("typing")
        .and_then(|m| m.getattr("get_type_hints"))
        .ok()?;
    let hints = get_type_hints.call1((&class_bound,)).ok()?;
    let field_type = hints.get_item(field_name).ok()?;

    // Extract the class name from the type (handling Optional, etc.)
    extract_type_class_name(&field_type)
}

/// Extract the class name from a Python type, handling generics like Optional[X].
fn extract_type_class_name(field_type: &Bound<'_, PyAny>) -> Option<String> {
    // Check for generic types with __origin__ (Optional, List, etc.)
    if field_type.getattr("__origin__").is_ok() {
        if let Ok(args) = field_type.getattr("__args__") {
            if let Ok(args_tuple) = args.cast::<pyo3::types::PyTuple>() {
                for arg in args_tuple.iter() {
                    // Skip NoneType
                    if let Ok(name) = arg.getattr("__name__") {
                        if let Ok(n) = name.extract::<String>() {
                            if n != "NoneType" {
                                return Some(n);
                            }
                        }
                    }
                    // Recurse into nested generics
                    if let Some(result) = extract_type_class_name(&arg) {
                        return Some(result);
                    }
                }
            }
        }
        return None;
    }

    // Simple class with __name__
    if let Ok(name) = field_type.getattr("__name__") {
        if let Ok(n) = name.extract::<String>() {
            if n != "NoneType" {
                return Some(n);
            }
        }
    }

    None
}

/// Infer the type of a field or property from the class registry.
pub(crate) fn infer_field_type(
    py: Python<'_>,
    class_name: &str,
    field_name: &str,
) -> PyResult<InferredType> {
    let class_ref: Option<Py<PyAny>> = {
        let registry = CLASS_REGISTRY.read().unwrap();
        if let Some(ref map) = *registry {
            map.get(class_name).map(|c| c.clone_ref(py))
        } else {
            None
        }
    };

    let Some(class) = class_ref else {
        return Ok(InferredType::Unknown);
    };

    let class_bound = class.bind(py);

    // First, try to get from type hints (annotated fields)
    let get_type_hints = py
        .import("typing")
        .and_then(|m| m.getattr("get_type_hints"));
    if let Ok(get_type_hints) = get_type_hints {
        let hints = get_type_hints.call1((&class_bound,));
        if let Ok(hints) = hints {
            if let Ok(field_type) = hints.get_item(field_name) {
                let type_name = get_concrete_type_name(&field_type);
                if let Some(inferred) = type_name_to_inferred_type(type_name.as_deref()) {
                    return Ok(inferred);
                }
            }
        }
    }

    // Not found in type hints - check if it's a property
    if let Ok(attr) = class_bound.getattr(field_name) {
        // Check if it's a property object
        let builtins = py.import("builtins")?;
        let property_type = builtins.getattr("property")?;
        if attr.is_instance(&property_type)? {
            // It's a property - get the return type from the getter's annotations
            if let Ok(fget) = attr.getattr("fget") {
                if let Ok(annotations) = fget.getattr("__annotations__") {
                    if let Ok(return_type) = annotations.get_item("return") {
                        let type_name = get_concrete_type_name(&return_type);
                        if let Some(inferred) = type_name_to_inferred_type(type_name.as_deref()) {
                            return Ok(inferred);
                        }
                    }
                }
            }
        }
    }

    Ok(InferredType::Unknown)
}

/// Convert a type name string to InferredType.
pub(crate) fn type_name_to_inferred_type(type_name: Option<&str>) -> Option<InferredType> {
    match type_name {
        Some("datetime") => Some(InferredType::I64),
        Some("timedelta") => Some(InferredType::I64),
        Some("int") => Some(InferredType::I32),
        Some("float") => Some(InferredType::F64),
        Some("bool") => Some(InferredType::Bool),
        Some("str") => Some(InferredType::String),
        _ => None,
    }
}

/// Try to inline a property access.
///
/// If the attribute is a property on the class, analyze the property getter
/// and substitute the base object for `self`.
pub(crate) fn try_inline_property(
    py: Python<'_>,
    class_name: &str,
    property_name: &str,
    base_expr: &Expression,
) -> PyResult<Option<Expression>> {
    let class_ref: Option<Py<PyAny>> = {
        let registry = CLASS_REGISTRY.read().unwrap();
        if let Some(ref map) = *registry {
            map.get(class_name).map(|c| c.clone_ref(py))
        } else {
            None
        }
    };

    let Some(class) = class_ref else {
        return Ok(None);
    };

    let class_bound = class.bind(py);

    // Check if the attribute is a property
    let Ok(attr) = class_bound.getattr(property_name) else {
        return Ok(None);
    };

    let builtins = py.import("builtins")?;
    let property_type = builtins.getattr("property")?;

    if !attr.is_instance(&property_type)? {
        // Not a property - it's a regular field
        return Ok(None);
    }

    // It's a property - get the getter function
    let Ok(fget) = attr.getattr("fget") else {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Property {}.{} has no getter",
            class_name, property_name
        )));
    };

    // Convert to Py<PyAny> for analyze_method_body
    let fget_py: Py<PyAny> = fget.clone().unbind();

    // Analyze the property getter body
    let property_body = crate::lambda_analyzer::analyze_method_body(py, &fget_py, class_name)?;

    // Substitute the base expression for Param(0) (self)
    let inlined = crate::lambda_analyzer::substitute_param(property_body, 0, base_expr);
    Ok(Some(inlined))
}

/// Get the concrete type name from a Python type, handling generics.
pub(crate) fn get_concrete_type_name(field_type: &Bound<'_, PyAny>) -> Option<String> {
    // Check for generic types with __origin__
    if field_type.getattr("__origin__").is_ok() {
        if let Ok(args) = field_type.getattr("__args__") {
            if let Ok(args_tuple) = args.cast::<pyo3::types::PyTuple>() {
                for arg in args_tuple.iter() {
                    if let Ok(name) = arg.getattr("__name__") {
                        if let Ok(n) = name.extract::<String>() {
                            if n != "NoneType" {
                                return Some(n);
                            }
                        }
                    }
                    // Recurse
                    if let Some(result) = get_concrete_type_name(&arg) {
                        return Some(result);
                    }
                }
            }
        }
        return None;
    }

    // Simple class
    if let Ok(name) = field_type.getattr("__name__") {
        if let Ok(n) = name.extract::<String>() {
            return Some(n);
        }
    }

    None
}

/// Infer the type of an already-converted Expression.
///
/// This is used when we need to determine the type of an expression
/// after it has been converted from AST. Useful for local variable
/// substitution scenarios.
pub fn infer_expression_type(expr: &Expression) -> InferredType {
    match expr {
        // Boolean
        Expression::BoolLiteral { .. }
        | Expression::Not { .. }
        | Expression::And { .. }
        | Expression::Or { .. }
        | Expression::Eq { .. }
        | Expression::Ne { .. }
        | Expression::Lt { .. }
        | Expression::Le { .. }
        | Expression::Gt { .. }
        | Expression::Ge { .. }
        | Expression::Eq64 { .. }
        | Expression::Ne64 { .. }
        | Expression::Lt64 { .. }
        | Expression::Le64 { .. }
        | Expression::Gt64 { .. }
        | Expression::Ge64 { .. }
        | Expression::IsNull { .. }
        | Expression::IsNotNull { .. }
        | Expression::IsNull64 { .. }
        | Expression::IsNotNull64 { .. } => InferredType::Bool,

        // Float
        Expression::FloatLiteral { .. }
        | Expression::FloatAdd { .. }
        | Expression::FloatSub { .. }
        | Expression::FloatMul { .. }
        | Expression::FloatDiv { .. }
        | Expression::Sqrt { .. }
        | Expression::FloatAbs { .. }
        | Expression::Sin { .. }
        | Expression::Cos { .. }
        | Expression::Asin { .. }
        | Expression::Acos { .. }
        | Expression::Atan { .. }
        | Expression::Atan2 { .. }
        | Expression::Radians { .. }
        | Expression::IntToFloat { .. } => InferredType::F64,

        // I64
        Expression::Int64Literal { .. }
        | Expression::Add64 { .. }
        | Expression::Sub64 { .. }
        | Expression::Mul64 { .. }
        | Expression::Div64 { .. }
        | Expression::IfThenElse64 { .. } => InferredType::I64,

        // I32
        Expression::IntLiteral { .. }
        | Expression::Add { .. }
        | Expression::Sub { .. }
        | Expression::Mul { .. }
        | Expression::Div { .. }
        | Expression::Length { .. }
        | Expression::FloatToInt { .. }
        | Expression::Round { .. }
        | Expression::Floor { .. }
        | Expression::Ceil { .. } => InferredType::I32,

        // String
        Expression::StringLiteral { .. } => InferredType::String,

        // Null
        Expression::Null => InferredType::Null,

        // IfThenElse - check branches
        Expression::IfThenElse {
            then_branch,
            else_branch,
            ..
        } => infer_expression_type(then_branch).promote(infer_expression_type(else_branch)),

        // FieldAccess - use the stored field_type
        Expression::FieldAccess { field_type, .. } => match field_type {
            solverforge_core::wasm::WasmFieldType::I32 => InferredType::I32,
            solverforge_core::wasm::WasmFieldType::I64 => InferredType::I64,
            solverforge_core::wasm::WasmFieldType::F64 => InferredType::F64,
            solverforge_core::wasm::WasmFieldType::Bool => InferredType::Bool,
            solverforge_core::wasm::WasmFieldType::String => InferredType::String,
            solverforge_core::wasm::WasmFieldType::Object => InferredType::Unknown,
        },

        // Default to Unknown for complex cases
        _ => InferredType::Unknown,
    }
}

/// Extract argument names from Python AST arguments node.
pub(crate) fn extract_arg_names(_py: Python<'_>, args: &Bound<'_, PyAny>) -> PyResult<Vec<String>> {
    let arg_list = args.getattr("args")?;
    let list = arg_list.cast::<PyList>()?;

    let mut names = Vec::new();
    for arg in list.iter() {
        let name: String = arg.getattr("arg")?.extract()?;
        names.push(name);
    }

    Ok(names)
}
