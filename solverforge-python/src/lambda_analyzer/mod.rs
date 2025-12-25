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

use ast_convert::{convert_ast_to_expression, extract_arg_names};
pub use registry::{get_method_from_class, register_class};
use type_inference::get_field_type_and_register;

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
