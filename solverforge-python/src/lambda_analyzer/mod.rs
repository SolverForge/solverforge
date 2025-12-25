//! Lambda analysis for converting Python lambdas to Expression trees.
//!
//! This module provides the infrastructure to analyze Python lambdas at definition time
//! and convert them to `Expression` trees that can be compiled to WASM.
//!
//! **Requires Python 3.13+** - uses modern AST structure (Constant nodes only,
//! no legacy Num/Str/NameConstant support).
//!
//! # Supported Patterns
//!
//! - Field access: `lambda x: x.field`
//! - Null checks: `lambda x: x.room is not None`
//! - Comparisons: `lambda x: x.count > 5`
//! - Boolean ops: `lambda x: x.a and x.b`
//! - Arithmetic: `lambda x: x.value + 10`
//! - Multi-param: `lambda a, b: a.room == b.room`
//!
//! # Example
//!
//! ```python
//! # These lambdas can be analyzed:
//! Joiners.equal(lambda lesson: lesson.timeslot)
//! factory.for_each("Lesson").filter(lambda l: l.room is not None)
//! ```

mod ast_convert;
mod conditionals;
mod constants;
mod loops;
mod method_analysis;
mod patterns;
mod registry;
mod sequential;
#[cfg(test)]
mod tests;
mod type_inference;

use ast_convert::{
    convert_binop_to_expression, convert_boolop_to_expression, convert_compare_to_expression,
    convert_constant_to_expression, convert_unaryop_to_expression, extract_arg_names,
};
use constants::get_class_constant;
pub use registry::{get_method_from_class, register_class};
use type_inference::{get_field_type_and_register, infer_expression_class};

use pyo3::prelude::*;
use pyo3::types::PyList;
use solverforge_core::constraints::WasmFunction;
use solverforge_core::wasm::Expression;
use std::sync::atomic::{AtomicU64, Ordering};

/// Global counter for generating unique lambda names.
static LAMBDA_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Generate a unique name for a lambda function.
///
/// Each call returns a unique name like "equal_map_0", "equal_map_1", etc.
pub fn generate_lambda_name(prefix: &str) -> String {
    let id = LAMBDA_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("{}_{}", prefix, id)
}

/// Information about an analyzed lambda.
///
/// This stores pure Rust data - no Python references. The Python callable
/// is only used during analysis and then discarded.
#[derive(Clone, Debug)]
pub struct LambdaInfo {
    /// Generated unique name for this lambda.
    pub name: String,
    /// Number of parameters the lambda expects.
    pub param_count: usize,
    /// Optional class name hint for type inference.
    pub class_hint: Option<String>,
    /// The analyzed expression.
    pub expression: Expression,
}

impl LambdaInfo {
    /// Create a new LambdaInfo from a Python callable.
    ///
    /// The callable is analyzed immediately and then discarded.
    /// Only the resulting Expression is kept - no Python references are stored.
    ///
    /// # Arguments
    /// * `py` - Python interpreter
    /// * `callable` - The Python lambda/function to analyze
    /// * `prefix` - Prefix for generating unique names (e.g., "filter", "map")
    /// * `class_hint` - The class name for type inference (required for method inlining)
    pub fn new(
        py: Python<'_>,
        callable: Py<PyAny>,
        prefix: &str,
        class_hint: &str,
    ) -> PyResult<Self> {
        let name = generate_lambda_name(prefix);
        let param_count = get_param_count(py, &callable)?;

        // Analyze the lambda immediately - callable is only used here
        let expression = analyze_lambda(py, &callable, param_count, class_hint)?;

        // Return pure Rust struct - no Python references
        Ok(Self {
            name,
            param_count,
            class_hint: Some(class_hint.to_string()),
            expression,
        })
    }

    /// Convert to a WasmFunction reference.
    pub fn to_wasm_function(&self) -> WasmFunction {
        WasmFunction::new(&self.name)
    }

    /// Get the analyzed expression.
    pub fn get_expression(&self) -> &Expression {
        &self.expression
    }
}

// ============================================================================
// Method Analysis Wrappers
// ============================================================================

/// Get the number of parameters from a Python callable.
fn get_param_count(py: Python<'_>, callable: &Py<PyAny>) -> PyResult<usize> {
    method_analysis::get_param_count(py, callable)
}

/// Analyze a Python method body and convert to an Expression tree.
pub fn analyze_method_body(
    py: Python<'_>,
    method: &Py<PyAny>,
    class_hint: &str,
) -> PyResult<Expression> {
    method_analysis::analyze_method_body(
        py,
        method,
        class_hint,
        convert_ast_to_expression,
        build_method_call_expr,
        accumulation_pattern_wrapper,
        sequential_pattern_wrapper,
    )
}

/// Wrapper for sequential::try_extract_sequential_expression_pattern.
fn sequential_pattern_wrapper(
    py: Python<'_>,
    stmts: &[Bound<'_, PyAny>],
    arg_names: &[String],
    class_hint: &str,
) -> PyResult<Expression> {
    sequential::try_extract_sequential_expression_pattern(
        py,
        stmts,
        arg_names,
        class_hint,
        convert_ast_to_expression,
        build_method_call_expr,
    )
}

// ============================================================================
// Expression Substitution
// ============================================================================

/// Substitute a parameter in an expression tree with another expression.
///
/// This is a thin wrapper around `Expression::substitute_param` from solverforge-core.
/// The implementation is in core so it can be shared across language bindings (Python, JS, etc.).
///
/// See `solverforge_core::wasm::Expression::substitute_param` for full documentation.
#[inline]
pub fn substitute_param(expr: Expression, from_index: u32, substitute: &Expression) -> Expression {
    expr.substitute_param(from_index, substitute)
}

// ============================================================================
// Lambda Analysis
// ============================================================================

/// Analyze a Python lambda and convert to an Expression tree.
///
/// This function uses Python's AST module to parse the lambda and convert it
/// to a solverforge Expression tree. The callable is used only for analysis
/// and is not stored.
///
/// # Errors
///
/// Returns an error with a clear message if the lambda pattern is not supported.
fn analyze_lambda(
    py: Python<'_>,
    callable: &Py<PyAny>,
    param_count: usize,
    class_hint: &str,
) -> PyResult<Expression> {
    let inspect = py.import("inspect")?;

    let source = inspect.call_method1("getsource", (callable,)).map_err(|_| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Cannot analyze lambda: source code unavailable. Lambdas must be defined in source files.",
        )
    })?;
    let source_str: String = source.extract()?;
    analyze_lambda_source(py, &source_str, param_count, class_hint)
}

/// Analyze lambda from source code.
fn analyze_lambda_source(
    py: Python<'_>,
    source: &str,
    param_count: usize,
    class_hint: &str,
) -> PyResult<Expression> {
    let ast = py.import("ast")?;

    // Try to extract just the lambda expression from the source
    // Source might be like ".filter(lambda x: x.field)" which isn't valid Python
    let lambda_source = extract_lambda_from_source(source);

    // Parse the extracted lambda source
    let tree = ast.call_method1("parse", (&lambda_source,)).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Cannot parse lambda source '{}': {}",
            lambda_source, e
        ))
    })?;

    // Walk the AST to find the lambda expression
    let body = tree.getattr("body")?;

    // Extract lambda node and convert to Expression
    extract_lambda_expression(py, &body, param_count, class_hint)?.ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Cannot extract lambda expression from source: {}",
            lambda_source
        ))
    })
}

/// Extract the lambda expression from source that may contain surrounding code.
///
/// Handles cases like:
/// - ".filter(lambda x: x.field)" -> "lambda x: x.field"
/// - "    .penalize(HardSoftScore.ONE_HARD, lambda v: v.demand)" -> "lambda v: v.demand"
fn extract_lambda_from_source(source: &str) -> String {
    // Find "lambda" keyword
    if let Some(lambda_start) = source.find("lambda") {
        let rest = &source[lambda_start..];

        // Find the end of the lambda - balance parentheses
        let mut depth = 0;
        let mut end_idx = rest.len();
        let mut in_string = false;
        let mut string_char = ' ';
        let mut past_colon = false; // Commas before colon are param separators

        for (i, c) in rest.char_indices() {
            // Handle string literals
            if (c == '"' || c == '\'') && !in_string {
                in_string = true;
                string_char = c;
            } else if c == string_char && in_string {
                in_string = false;
            }

            if in_string {
                continue;
            }

            match c {
                ':' if depth == 0 && !past_colon => {
                    past_colon = true;
                }
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => {
                    if depth == 0 {
                        // Found closing paren that ends the lambda
                        end_idx = i;
                        break;
                    }
                    depth -= 1;
                }
                ',' if depth == 0 && past_colon => {
                    // Comma at depth 0 AFTER colon ends the lambda argument
                    end_idx = i;
                    break;
                }
                _ => {}
            }
        }

        let lambda_expr = rest[..end_idx].trim();

        // Wrap in a statement for parsing: "_ = lambda x: x.field"
        format!("_ = {}", lambda_expr)
    } else {
        // No lambda found, return original
        source.to_string()
    }
}

/// Extract Expression from Python AST node.
fn extract_lambda_expression(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    _param_count: usize,
    class_hint: &str,
) -> PyResult<Option<Expression>> {
    let node_type = node.get_type().name()?.to_string();

    match node_type.as_str() {
        "list" => {
            // Body is a list, find lambda in it
            let list = node.cast::<PyList>()?;
            for item in list.iter() {
                if let Some(expr) = extract_lambda_expression(py, &item, _param_count, class_hint)?
                {
                    return Ok(Some(expr));
                }
            }
            Ok(None)
        }

        "Expr" => {
            // Expression statement wrapper
            let value = node.getattr("value")?;
            extract_lambda_expression(py, &value, _param_count, class_hint)
        }

        "Assign" => {
            // Assignment statement - check the value
            let value = node.getattr("value")?;
            extract_lambda_expression(py, &value, _param_count, class_hint)
        }

        "Lambda" => {
            // Found the lambda - analyze its body
            let body = node.getattr("body")?;
            let args = node.getattr("args")?;
            let arg_names = extract_arg_names(py, &args)?;

            convert_ast_to_expression(py, &body, &arg_names, class_hint)
        }

        "Call" => {
            // Function call - might wrap a lambda
            let args_node = node.getattr("args")?;
            let args_list = args_node.cast::<PyList>()?;

            for arg in args_list.iter() {
                if let Some(expr) = extract_lambda_expression(py, &arg, _param_count, class_hint)? {
                    return Ok(Some(expr));
                }
            }
            Ok(None)
        }

        _ => Ok(None),
    }
}

/// Wrapper for loops::try_extract_accumulation_pattern matching the AccumFn signature.
fn accumulation_pattern_wrapper(
    py: Python<'_>,
    body_list: &Bound<'_, PyList>,
    arg_names: &[String],
    class_hint: &str,
) -> PyResult<Expression> {
    loops::try_extract_accumulation_pattern(
        py,
        body_list,
        arg_names,
        class_hint,
        convert_ast_to_expression,
        build_method_call_expr,
    )
}

// ============================================================================
// Method Call Inlining
// ============================================================================

/// Build an inlined method call expression.
///
/// Tries to inline the method body. Returns an error if inlining fails.
fn build_method_call_expr(
    py: Python<'_>,
    base: Expression,
    method_name: &str,
    args: &[Expression],
    class_hint: &str,
) -> PyResult<Expression> {
    // For methods, we need to inline them
    // Get the base class from the expression by looking up field types
    let base_class = match &base {
        Expression::FieldAccess {
            class_name,
            field_name,
            ..
        } => {
            // Look up the field type from the class and register it
            if let Some(field_class) = get_field_type_and_register(py, class_name, field_name)? {
                field_class
            } else {
                class_hint.to_string()
            }
        }
        _ => class_hint.to_string(),
    };

    // Try to inline the method
    if let Some(method) = get_method_from_class(py, &base_class, method_name) {
        if let Ok(method_body) = analyze_method_body(py, &method, &base_class) {
            // Substitute self (Param 0) with base
            let mut inlined = substitute_param(method_body, 0, &base);

            // Substitute other args
            for (i, arg) in args.iter().enumerate() {
                inlined = substitute_param(inlined, (i + 1) as u32, arg);
            }

            return Ok(inlined);
        }
    }

    // Inlining failed - return error
    Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
        "Cannot inline method {}.{}() - method not found or inlining failed",
        base_class, method_name
    )))
}

// ============================================================================
// AST to Expression Conversion
// ============================================================================

/// Convert Python AST node to Expression tree.
fn convert_ast_to_expression(
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
                        // Handle math module functions
                        let args_node = node.getattr("args")?;
                        let args_list = args_node.cast::<PyList>()?;

                        let mut call_args = Vec::new();
                        for arg in args_list.iter() {
                            if let Some(arg_expr) =
                                convert_ast_to_expression(py, &arg, arg_names, class_hint)?
                            {
                                call_args.push(arg_expr);
                            } else {
                                return Ok(None);
                            }
                        }

                        return match method_name.as_str() {
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
                        };
                    }
                }

                // Method call: obj.method()
                // Get the object expression
                if let Some(obj_expr) =
                    convert_ast_to_expression(py, &value, arg_names, class_hint)?
                {
                    // Convert method call arguments
                    let args_node = node.getattr("args")?;
                    let args_list = args_node.cast::<PyList>()?;
                    let mut call_args = vec![obj_expr.clone()];

                    for arg in args_list.iter() {
                        if let Some(arg_expr) =
                            convert_ast_to_expression(py, &arg, arg_names, class_hint)?
                        {
                            call_args.push(arg_expr);
                        }
                    }

                    // Determine the actual class of the object for method lookup
                    let obj_class = infer_expression_class(py, &obj_expr, class_hint)?
                        .unwrap_or_else(|| class_hint.to_string());

                    // Try to inline the method - look it up in the registry and analyze
                    if let Some(method) = get_method_from_class(py, &obj_class, &method_name) {
                        match analyze_method_body(py, &method, &obj_class) {
                            Ok(method_body) => {
                                // Substitute parameters: obj_expr becomes Param(0), call_args become Param(1), etc.
                                let mut inlined = method_body;

                                // Substitute method parameters with call arguments
                                // The object is Param(0) in the method, and obj_expr in the call
                                inlined = substitute_param(inlined, 0, &obj_expr);

                                // Substitute other parameters
                                for (i, arg) in args_list.iter().enumerate() {
                                    if let Some(arg_expr) =
                                        convert_ast_to_expression(py, &arg, arg_names, class_hint)?
                                    {
                                        // In the method, args start at Param(1)
                                        inlined =
                                            substitute_param(inlined, (i + 1) as u32, &arg_expr);
                                    }
                                }

                                return Ok(Some(inlined));
                            }
                            Err(e) => {
                                // Method couldn't be inlined - return error
                                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                                    format!(
                                        "Cannot inline method {}.{}(): {}",
                                        obj_class, method_name, e
                                    ),
                                ));
                            }
                        }
                    }

                    // Method not found in registry - return error
                    Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Cannot inline method {}.{}() - class not registered. Register the class with register_class() first.",
                        obj_class, method_name
                    )))
                } else {
                    Ok(None)
                }
            } else if func_type == "Name" {
                // Built-in function call like max(), min(), timedelta(), etc.
                let func_name: String = func.getattr("id")?.extract()?;
                let args_node = node.getattr("args")?;
                let args_list = args_node.cast::<PyList>()?;

                // Convert positional arguments to expressions
                let mut call_args = Vec::new();
                for arg in args_list.iter() {
                    if let Some(arg_expr) =
                        convert_ast_to_expression(py, &arg, arg_names, class_hint)?
                    {
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
                match func_name.as_str() {
                    "max" => {
                        if call_args.len() == 2 {
                            // max(a, b) as a ternary: a > b ? a : b
                            return Ok(Some(Expression::IfThenElse {
                                condition: Box::new(Expression::Gt {
                                    left: Box::new(call_args[0].clone()),
                                    right: Box::new(call_args[1].clone()),
                                }),
                                then_branch: Box::new(call_args[0].clone()),
                                else_branch: Box::new(call_args[1].clone()),
                            }));
                        }
                    }
                    "min" => {
                        if call_args.len() == 2 {
                            // min(a, b) as a ternary: a < b ? a : b
                            return Ok(Some(Expression::IfThenElse {
                                condition: Box::new(Expression::Lt {
                                    left: Box::new(call_args[0].clone()),
                                    right: Box::new(call_args[1].clone()),
                                }),
                                then_branch: Box::new(call_args[0].clone()),
                                else_branch: Box::new(call_args[1].clone()),
                            }));
                        }
                    }
                    "abs" => {
                        if call_args.len() == 1 {
                            // abs(a) as: a < 0 ? -a : a
                            return Ok(Some(Expression::IfThenElse {
                                condition: Box::new(Expression::Lt {
                                    left: Box::new(call_args[0].clone()),
                                    right: Box::new(Expression::IntLiteral { value: 0 }),
                                }),
                                then_branch: Box::new(Expression::Mul {
                                    left: Box::new(Expression::IntLiteral { value: -1 }),
                                    right: Box::new(call_args[0].clone()),
                                }),
                                else_branch: Box::new(call_args[0].clone()),
                            }));
                        }
                    }
                    "len" => {
                        // len(collection) -> Length expression
                        if call_args.len() == 1 {
                            return Ok(Some(Expression::Length {
                                collection: Box::new(call_args[0].clone()),
                            }));
                        }
                    }
                    "round" => {
                        // round(x) -> Round expression (WASM f64.nearest)
                        if call_args.len() == 1 {
                            return Ok(Some(Expression::Round {
                                operand: Box::new(call_args[0].clone()),
                            }));
                        }
                    }
                    "int" => {
                        // int(x) -> FloatToInt for float values
                        if call_args.len() == 1 {
                            return Ok(Some(Expression::FloatToInt {
                                operand: Box::new(call_args[0].clone()),
                            }));
                        }
                    }
                    "float" => {
                        // float(x) -> IntToFloat for int values
                        if call_args.len() == 1 {
                            return Ok(Some(Expression::IntToFloat {
                                operand: Box::new(call_args[0].clone()),
                            }));
                        }
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
                        return Ok(Some(Expression::IntLiteral {
                            value: total_seconds,
                        }));
                    }
                    _ => {
                        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                            "Cannot inline function call: {}()",
                            func_name
                        )));
                    }
                }
                Ok(None)
            } else {
                // Other types of calls - not supported for inlining
                Ok(None)
            }
        }

        _ => Ok(None),
    }
}
