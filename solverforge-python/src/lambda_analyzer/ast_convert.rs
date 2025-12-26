//! AST to Expression conversion.
//!
//! This module contains functions for converting Python AST nodes
//! to solverforge Expression trees.
//!
//! Type inference is performed FIRST by analyzing the AST structure,
//! then expressions are emitted ONCE with the correct types.

use pyo3::prelude::*;
use pyo3::types::PyList;
use solverforge_core::wasm::Expression;

use super::constants::get_class_constant;
use super::registry::{get_method_from_class, CLASS_REGISTRY};
use super::type_inference::infer_expression_class;

/// Inferred type from AST analysis.
///
/// This enum represents the type of a value as determined by analyzing
/// the AST structure BEFORE converting to expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferredType {
    /// 32-bit integer (default for integer literals and operations)
    I32,
    /// 64-bit integer (datetime fields, large integers, timedelta)
    I64,
    /// 64-bit floating point
    F64,
    /// Boolean
    Bool,
    /// String
    String,
    /// Null/None
    Null,
    /// Unknown type (couldn't be determined)
    Unknown,
}

impl InferredType {
    /// Promote two numeric types to a common type for binary operations.
    /// - If either is F64, result is F64
    /// - If either is I64, result is I64
    /// - Otherwise I32
    pub fn promote(self, other: InferredType) -> InferredType {
        match (self, other) {
            // Float propagates
            (InferredType::F64, _) | (_, InferredType::F64) => InferredType::F64,
            // I64 propagates
            (InferredType::I64, _) | (_, InferredType::I64) => InferredType::I64,
            // Both I32
            (InferredType::I32, InferredType::I32) => InferredType::I32,
            // Unknown cases - be conservative
            _ => InferredType::Unknown,
        }
    }
}

/// Expected type context for expression conversion.
///
/// This is derived from InferredType and passed to conversion functions
/// to ensure literals are emitted with the correct type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExpectedType {
    /// No specific expectation - use default types (I32 for integers)
    #[default]
    Any,
    /// Expect i64 (for datetime fields, large integers)
    I64,
    /// Expect f64 (for floating point)
    F64,
}

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
fn infer_base_class(
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

/// Infer the type of a field from the class registry.
fn infer_field_type(py: Python<'_>, class_name: &str, field_name: &str) -> PyResult<InferredType> {
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

    let get_type_hints = py
        .import("typing")
        .and_then(|m| m.getattr("get_type_hints"));
    let Ok(get_type_hints) = get_type_hints else {
        return Ok(InferredType::Unknown);
    };

    let hints = get_type_hints.call1((&class_bound,));
    let Ok(hints) = hints else {
        return Ok(InferredType::Unknown);
    };

    let field_type = hints.get_item(field_name);
    let Ok(field_type) = field_type else {
        return Ok(InferredType::Unknown);
    };

    // Check the type name
    let type_name = get_concrete_type_name(&field_type);

    match type_name.as_deref() {
        Some("datetime") => Ok(InferredType::I64),
        Some("timedelta") => Ok(InferredType::I64),
        Some("int") => Ok(InferredType::I32),
        Some("float") => Ok(InferredType::F64),
        Some("bool") => Ok(InferredType::Bool),
        Some("str") => Ok(InferredType::String),
        _ => Ok(InferredType::Unknown),
    }
}

/// Get the concrete type name from a Python type, handling generics.
fn get_concrete_type_name(field_type: &Bound<'_, PyAny>) -> Option<String> {
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
        | Expression::IsNotNull { .. } => InferredType::Bool,

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
        let arg_name: String = arg.getattr("arg")?.extract()?;
        names.push(arg_name);
    }

    Ok(names)
}

/// Convert Python Compare AST node to Expression.
///
/// Single-pass type inference:
/// 1. Infer types from AST nodes (without converting)
/// 2. Promote types to determine operation type
/// 3. Convert ONCE with correct expected type
pub(crate) fn convert_compare_to_expression(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    arg_names: &[String],
    class_hint: &str,
) -> PyResult<Option<Expression>> {
    let left_node = node.getattr("left")?;
    let ops_list = node.getattr("ops")?.cast::<PyList>()?.clone();
    let comparators_list = node.getattr("comparators")?.cast::<PyList>()?.clone();

    let ops: Vec<Bound<'_, PyAny>> = ops_list.iter().collect();
    let comparators: Vec<Bound<'_, PyAny>> = comparators_list.iter().collect();

    if ops.len() != 1 || comparators.len() != 1 {
        // Multiple comparisons (a < b < c) not directly supported
        return Ok(None);
    }

    let right_node = &comparators[0];

    // STEP 1: Infer types from AST (no conversion yet)
    let left_type = infer_ast_type(py, &left_node, arg_names, class_hint)?;
    let right_type = infer_ast_type(py, right_node, arg_names, class_hint)?;

    // STEP 2: Promote to determine operation type
    let promoted_type = left_type.promote(right_type);
    let use_i64 = promoted_type == InferredType::I64;

    // STEP 3: Convert ONCE with correct expected type
    let expected = if use_i64 {
        ExpectedType::I64
    } else {
        ExpectedType::Any
    };

    let left = convert_ast_to_expression(py, &left_node, arg_names, class_hint, expected)?;
    let right = convert_ast_to_expression(py, right_node, arg_names, class_hint, expected)?;

    let (Some(left), Some(right)) = (left, right) else {
        return Ok(None);
    };

    let op_type = ops[0].get_type().name()?.to_string();

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

/// Convert Python BoolOp AST node (and/or) to Expression.
///
/// Boolean operations produce i32 (boolean) results, so operands
/// are converted with ExpectedType::Any (they're boolean expressions).
pub(crate) fn convert_boolop_to_expression(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    arg_names: &[String],
    class_hint: &str,
) -> PyResult<Option<Expression>> {
    let op = node.getattr("op")?;
    let values_list = node.getattr("values")?.cast::<PyList>()?.clone();
    let values: Vec<Bound<'_, PyAny>> = values_list.iter().collect();

    if values.len() < 2 {
        return Ok(None);
    }

    let op_type = op.get_type().name()?.to_string();

    // Convert all operands - boolean context, no type propagation needed
    let mut exprs: Vec<Expression> = Vec::new();
    for val in values.iter() {
        if let Some(expr) =
            convert_ast_to_expression(py, val, arg_names, class_hint, ExpectedType::Any)?
        {
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
///
/// Single-pass type inference:
/// 1. Infer type from operand AST
/// 2. Convert ONCE with correct expected type
/// 3. Emit correct expression based on inferred type
pub(crate) fn convert_unaryop_to_expression(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    arg_names: &[String],
    class_hint: &str,
    expected_type: ExpectedType,
) -> PyResult<Option<Expression>> {
    let op = node.getattr("op")?;
    let operand = node.getattr("operand")?;
    let op_type = op.get_type().name()?.to_string();

    // STEP 1: Infer type from operand AST
    let operand_type = infer_ast_type(py, &operand, arg_names, class_hint)?;

    // Determine the actual type to use (considering expected type context)
    let actual_type = match expected_type {
        ExpectedType::I64 => InferredType::I64,
        ExpectedType::F64 => InferredType::F64,
        ExpectedType::Any => operand_type,
    };

    // STEP 2: Convert ONCE with correct expected type
    let conversion_expected = match actual_type {
        InferredType::I64 => ExpectedType::I64,
        InferredType::F64 => ExpectedType::F64,
        _ => ExpectedType::Any,
    };

    let Some(operand_expr) =
        convert_ast_to_expression(py, &operand, arg_names, class_hint, conversion_expected)?
    else {
        return Ok(None);
    };

    // STEP 3: Emit correct expression based on inferred type
    let expr = match op_type.as_str() {
        "Not" => Expression::Not {
            operand: Box::new(operand_expr),
        },
        "USub" => {
            // Unary minus: -x implemented as 0 - x
            match actual_type {
                InferredType::I64 => Expression::Sub64 {
                    left: Box::new(Expression::Int64Literal { value: 0 }),
                    right: Box::new(operand_expr),
                },
                InferredType::F64 => Expression::FloatSub {
                    left: Box::new(Expression::FloatLiteral { value: 0.0 }),
                    right: Box::new(operand_expr),
                },
                _ => Expression::Sub {
                    left: Box::new(Expression::IntLiteral { value: 0 }),
                    right: Box::new(operand_expr),
                },
            }
        }
        _ => return Ok(None),
    };
    Ok(Some(expr))
}

/// Convert Python BinOp AST node to Expression.
///
/// Single-pass type inference:
/// 1. Infer types from AST nodes (without converting)
/// 2. Promote types to determine operation type
/// 3. Convert ONCE with correct expected type
pub(crate) fn convert_binop_to_expression(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    arg_names: &[String],
    class_hint: &str,
) -> PyResult<Option<Expression>> {
    let op = node.getattr("op")?;
    let left_node = node.getattr("left")?;
    let right_node = node.getattr("right")?;
    let op_type = op.get_type().name()?.to_string();

    // STEP 1: Infer types from AST (no conversion yet)
    let left_type = infer_ast_type(py, &left_node, arg_names, class_hint)?;
    let right_type = infer_ast_type(py, &right_node, arg_names, class_hint)?;

    // STEP 2: Promote to determine operation type
    // Python `/` always produces float regardless of operand types
    let promoted_type = if op_type == "Div" {
        InferredType::F64
    } else {
        left_type.promote(right_type)
    };

    let use_float = promoted_type == InferredType::F64;
    let use_i64 = promoted_type == InferredType::I64;

    // STEP 3: Convert ONCE with correct expected type
    let expected = match promoted_type {
        InferredType::F64 => ExpectedType::F64,
        InferredType::I64 => ExpectedType::I64,
        _ => ExpectedType::Any,
    };

    let l = convert_ast_to_expression(py, &left_node, arg_names, class_hint, expected)?;
    let r = convert_ast_to_expression(py, &right_node, arg_names, class_hint, expected)?;

    let (Some(l), Some(r)) = (l, r) else {
        return Ok(None);
    };

    let expr = match op_type.as_str() {
        "Add" => {
            if use_float {
                Expression::FloatAdd {
                    left: Box::new(l),
                    right: Box::new(r),
                }
            } else if use_i64 {
                Expression::Add64 {
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
            if use_float {
                Expression::FloatSub {
                    left: Box::new(l),
                    right: Box::new(r),
                }
            } else if use_i64 {
                Expression::Sub64 {
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
            if use_float {
                Expression::FloatMul {
                    left: Box::new(l),
                    right: Box::new(r),
                }
            } else if use_i64 {
                Expression::Mul64 {
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
        "FloorDiv" => {
            if use_i64 {
                Expression::Div64 {
                    left: Box::new(l),
                    right: Box::new(r),
                }
            } else {
                Expression::Div {
                    left: Box::new(l),
                    right: Box::new(r),
                }
            }
        }
        _ => return Ok(None),
    };

    Ok(Some(expr))
}

/// Convert Python constant to Expression.
///
/// The `expected_type` parameter guides literal type selection:
/// - `ExpectedType::I64` produces `Int64Literal` for integers
/// - `ExpectedType::F64` produces `FloatLiteral` (converting int if needed)
/// - `ExpectedType::Any` uses default types (I32 for integers)
pub(crate) fn convert_constant_to_expression(
    node: &Bound<'_, PyAny>,
    expected_type: ExpectedType,
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
                // Choose literal type based on expected type context
                match expected_type {
                    ExpectedType::I64 => Ok(Some(Expression::Int64Literal { value: i })),
                    ExpectedType::F64 => Ok(Some(Expression::FloatLiteral { value: i as f64 })),
                    ExpectedType::Any => Ok(Some(Expression::IntLiteral { value: i })),
                }
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
///
/// The `expected_type` parameter provides type context from the parent expression,
/// enabling proper literal type selection without post-hoc conversions.
pub(crate) fn convert_ast_to_expression(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    arg_names: &[String],
    class_hint: &str,
    expected_type: ExpectedType,
) -> PyResult<Option<Expression>> {
    let node_type = node.get_type().name()?.to_string();
    let class_name = class_hint.to_string();

    match node_type.as_str() {
        "Attribute" => {
            // Field access: x.field or class constant: x.CONSTANT
            let value = node.getattr("value")?;
            let attr: String = node.getattr("attr")?.extract()?;

            if let Some(base_expr) =
                convert_ast_to_expression(py, &value, arg_names, class_hint, ExpectedType::Any)?
            {
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
            convert_compare_to_expression(py, node, arg_names, class_hint)
        }

        "BoolOp" => {
            // Boolean operation: and, or
            convert_boolop_to_expression(py, node, arg_names, class_hint)
        }

        "UnaryOp" => {
            // Unary operation: not
            convert_unaryop_to_expression(py, node, arg_names, class_hint, expected_type)
        }

        "BinOp" => {
            // Binary operation: +, -, *, /
            convert_binop_to_expression(py, node, arg_names, class_hint)
        }

        "Constant" => {
            // Literal value - pass expected_type for proper literal selection
            convert_constant_to_expression(node, expected_type)
        }

        "Call" => {
            // Method call: obj.method() or function() or module.function()
            convert_call_to_expression(py, node, arg_names, class_hint, expected_type)
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
    expected_type: ExpectedType,
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
        return convert_builtin_call(py, node, &func_name, arg_names, class_hint, expected_type);
    }

    // Other types of calls - not supported for inlining
    Ok(None)
}

/// Convert math module function call.
///
/// Math functions work with floats, so arguments are converted with F64 context.
fn convert_math_call(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    method_name: &str,
    arg_names: &[String],
    class_hint: &str,
) -> PyResult<Option<Expression>> {
    let args_node = node.getattr("args")?;
    let args_list = args_node.cast::<PyList>()?;

    // Math functions expect float arguments
    let mut call_args = Vec::new();
    for arg in args_list.iter() {
        if let Some(arg_expr) =
            convert_ast_to_expression(py, &arg, arg_names, class_hint, ExpectedType::F64)?
        {
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
    // Get the object expression (no specific type expectation for object references)
    let Some(obj_expr) =
        convert_ast_to_expression(py, value, arg_names, class_hint, ExpectedType::Any)?
    else {
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

                // Substitute other parameters (using Any since we don't know method signature types)
                for (i, arg) in args_list.iter().enumerate() {
                    if let Some(arg_expr) = convert_ast_to_expression(
                        py,
                        &arg,
                        arg_names,
                        class_hint,
                        ExpectedType::Any,
                    )? {
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
///
/// Single-pass type inference:
/// 1. Infer types from AST arguments (without converting)
/// 2. Promote types to determine operation type
/// 3. Convert ONCE with correct expected type
fn convert_builtin_call(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    func_name: &str,
    arg_names: &[String],
    class_hint: &str,
    _expected_type: ExpectedType,
) -> PyResult<Option<Expression>> {
    let args_node = node.getattr("args")?;
    let args_list = args_node.cast::<PyList>()?;

    // STEP 1: Infer types from AST (no conversion yet)
    let mut arg_types = Vec::new();
    for arg in args_list.iter() {
        let arg_type = infer_ast_type(py, &arg, arg_names, class_hint)?;
        arg_types.push(arg_type);
    }

    // Parse keyword arguments (for timedelta)
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

    // STEP 2: Promote types to determine operation type
    let promoted_type = arg_types
        .iter()
        .copied()
        .reduce(|a, b| a.promote(b))
        .unwrap_or(InferredType::I32);
    let use_i64 = promoted_type == InferredType::I64;

    // STEP 3: Convert ONCE with correct expected type
    let expected = if use_i64 {
        ExpectedType::I64
    } else {
        ExpectedType::Any
    };

    let mut call_args = Vec::new();
    for arg in args_list.iter() {
        if let Some(arg_expr) =
            convert_ast_to_expression(py, &arg, arg_names, class_hint, expected)?
        {
            call_args.push(arg_expr);
        } else {
            return Ok(None);
        }
    }

    // Handle specific built-in functions
    match func_name {
        "max" if call_args.len() == 2 => {
            // max(a, b) as a ternary: a > b ? a : b
            if use_i64 {
                Ok(Some(Expression::IfThenElse64 {
                    condition: Box::new(Expression::Gt64 {
                        left: Box::new(call_args[0].clone()),
                        right: Box::new(call_args[1].clone()),
                    }),
                    then_branch: Box::new(call_args[0].clone()),
                    else_branch: Box::new(call_args[1].clone()),
                }))
            } else {
                Ok(Some(Expression::IfThenElse {
                    condition: Box::new(Expression::Gt {
                        left: Box::new(call_args[0].clone()),
                        right: Box::new(call_args[1].clone()),
                    }),
                    then_branch: Box::new(call_args[0].clone()),
                    else_branch: Box::new(call_args[1].clone()),
                }))
            }
        }
        "min" if call_args.len() == 2 => {
            // min(a, b) as a ternary: a < b ? a : b
            if use_i64 {
                Ok(Some(Expression::IfThenElse64 {
                    condition: Box::new(Expression::Lt64 {
                        left: Box::new(call_args[0].clone()),
                        right: Box::new(call_args[1].clone()),
                    }),
                    then_branch: Box::new(call_args[0].clone()),
                    else_branch: Box::new(call_args[1].clone()),
                }))
            } else {
                Ok(Some(Expression::IfThenElse {
                    condition: Box::new(Expression::Lt {
                        left: Box::new(call_args[0].clone()),
                        right: Box::new(call_args[1].clone()),
                    }),
                    then_branch: Box::new(call_args[0].clone()),
                    else_branch: Box::new(call_args[1].clone()),
                }))
            }
        }
        "abs" if call_args.len() == 1 => {
            // abs(a) as: a < 0 ? -a : a
            if use_i64 {
                Ok(Some(Expression::IfThenElse64 {
                    condition: Box::new(Expression::Lt64 {
                        left: Box::new(call_args[0].clone()),
                        right: Box::new(Expression::Int64Literal { value: 0 }),
                    }),
                    then_branch: Box::new(Expression::Mul64 {
                        left: Box::new(Expression::Int64Literal { value: -1 }),
                        right: Box::new(call_args[0].clone()),
                    }),
                    else_branch: Box::new(call_args[0].clone()),
                }))
            } else {
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
            // Convert timedelta to i64 seconds.
            // timedelta is always used with datetime fields (which are i64),
            // so we always emit Int64Literal to ensure type consistency.
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
            // Always use Int64Literal - timedelta is semantically a duration for datetime
            Ok(Some(Expression::Int64Literal {
                value: total_seconds,
            }))
        }
        _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Cannot inline function call: {}()",
            func_name
        ))),
    }
}
