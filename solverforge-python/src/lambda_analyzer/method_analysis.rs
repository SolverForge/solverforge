//! Method body analysis and inlining.
//!
//! This module handles analyzing Python method bodies and inlining method calls
//! into Expression trees.

use pyo3::prelude::*;
use pyo3::types::PyList;
use solverforge_core::wasm::Expression;

use super::ast_convert::extract_arg_names;
use super::conditionals;
use super::loops;

/// Function type for AST to Expression conversion.
pub type ConvertFn =
    fn(Python<'_>, &Bound<'_, PyAny>, &[String], &str) -> PyResult<Option<Expression>>;

/// Function type for building method call expressions.
pub type BuildMethodCallFn =
    fn(Python<'_>, Expression, &str, &[Expression], &str) -> PyResult<Expression>;

/// Function type for accumulation pattern extraction.
pub type AccumFn = fn(Python<'_>, &Bound<'_, PyList>, &[String], &str) -> PyResult<Expression>;

/// Function type for sequential expression pattern extraction.
pub type SequentialFn =
    fn(Python<'_>, &[Bound<'_, PyAny>], &[String], &str) -> PyResult<Expression>;

/// Get the number of parameters from a Python callable.
pub fn get_param_count(py: Python<'_>, callable: &Py<PyAny>) -> PyResult<usize> {
    let inspect = py.import("inspect")?;
    let sig = inspect.call_method1("signature", (callable,))?;
    let params = sig.getattr("parameters")?;
    let len = params.len()?;
    Ok(len)
}

/// Analyze a Python method body and convert to an Expression tree.
///
/// This reuses the bytecode analysis infrastructure but handles `self`
/// as parameter index 0. The returned Expression can be used to inline
/// method calls by substituting `self` with the calling object.
///
/// # Arguments
/// * `py` - Python interpreter
/// * `method` - The method callable to analyze
/// * `class_hint` - Class name for field type inference (required)
/// * `convert_fn` - Function to convert AST to Expression
/// * `build_method_call_fn` - Function to build method call expressions
/// * `accum_fn` - Function to extract accumulation patterns
/// * `sequential_fn` - Function to extract sequential expression patterns
///
/// # Returns
/// An Expression tree representing the method body, where:
/// - `Param { index: 0 }` represents `self`
/// - Other params are indexed 1, 2, etc.
pub fn analyze_method_body(
    py: Python<'_>,
    method: &Py<PyAny>,
    class_hint: &str,
    convert_fn: ConvertFn,
    build_method_call_fn: BuildMethodCallFn,
    accum_fn: AccumFn,
    sequential_fn: SequentialFn,
) -> PyResult<Expression> {
    let param_count = get_param_count(py, method)?;

    let inspect = py.import("inspect")?;
    let source = inspect.call_method1("getsource", (method,)).map_err(|_| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Cannot analyze method: source code unavailable. Methods must be defined in source files.",
        )
    })?;
    let source_str: String = source.extract()?;
    analyze_method_source(
        py,
        &source_str,
        param_count,
        class_hint,
        convert_fn,
        build_method_call_fn,
        accum_fn,
        sequential_fn,
    )
}

/// Analyze method from source code.
#[allow(clippy::too_many_arguments)]
fn analyze_method_source(
    py: Python<'_>,
    source: &str,
    param_count: usize,
    class_hint: &str,
    convert_fn: ConvertFn,
    build_method_call_fn: BuildMethodCallFn,
    accum_fn: AccumFn,
    sequential_fn: SequentialFn,
) -> PyResult<Expression> {
    let ast = py.import("ast")?;

    // Parse the method source
    // We need to handle indentation - dedent the source first
    let textwrap = py.import("textwrap")?;
    let dedented: String = textwrap.call_method1("dedent", (source,))?.extract()?;

    let tree = ast.call_method1("parse", (&dedented,)).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Cannot parse method source: {}",
            e
        ))
    })?;

    // Find the FunctionDef node and extract its return expression
    let body = tree.getattr("body")?;
    extract_method_return_expression(
        py,
        &body,
        param_count,
        class_hint,
        convert_fn,
        build_method_call_fn,
        accum_fn,
        sequential_fn,
    )
}

/// Extract the return expression from a method's AST.
#[allow(clippy::too_many_arguments, clippy::only_used_in_recursion)]
fn extract_method_return_expression(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    param_count: usize,
    class_hint: &str,
    convert_fn: ConvertFn,
    build_method_call_fn: BuildMethodCallFn,
    accum_fn: AccumFn,
    sequential_fn: SequentialFn,
) -> PyResult<Expression> {
    let node_type = node.get_type().name()?.to_string();

    match node_type.as_str() {
        "list" => {
            // Body is a list, find FunctionDef
            let list = node.cast::<PyList>()?;
            for item in list.iter() {
                let item_type = item.get_type().name()?.to_string();
                if item_type == "FunctionDef" || item_type == "AsyncFunctionDef" {
                    return extract_method_return_expression(
                        py,
                        &item,
                        param_count,
                        class_hint,
                        convert_fn,
                        build_method_call_fn,
                        accum_fn,
                        sequential_fn,
                    );
                }
            }
            Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Cannot analyze method: no function definition found",
            ))
        }

        "FunctionDef" | "AsyncFunctionDef" => {
            // Extract argument names including 'self'
            let args = node.getattr("args")?;
            let arg_names = extract_arg_names(py, &args)?;

            // Find return statement in function body
            let body = node.getattr("body")?;
            let body_list = body.cast::<PyList>()?;
            let stmts: Vec<Bound<'_, PyAny>> = body_list.iter().collect();

            for (i, stmt) in stmts.iter().enumerate() {
                let stmt_type = stmt.get_type().name()?.to_string();
                if stmt_type == "Return" {
                    let value = stmt.getattr("value")?;
                    if !value.is_none() {
                        if let Some(expr) = convert_fn(py, &value, &arg_names, class_hint)? {
                            return Ok(expr);
                        }
                    }
                }
                // Handle If statements with returns
                if stmt_type == "If" {
                    // First try standard if/else extraction
                    if let Ok(expr) =
                        conditionals::extract_if_else(py, stmt, &arg_names, class_hint, convert_fn)
                    {
                        return Ok(expr);
                    }
                    // Try if-early-return pattern: if condition returns, rest is else
                    if let Ok(expr) = conditionals::extract_early_return(
                        py,
                        stmt,
                        &stmts[i + 1..],
                        &arg_names,
                        class_hint,
                        convert_fn,
                        accum_fn,
                    ) {
                        return Ok(expr);
                    }
                }
            }

            // Try to recognize common patterns with loops (e.g., sum accumulation)
            if let Ok(expr) = loops::try_extract_accumulation_pattern(
                py,
                body_list,
                &arg_names,
                class_hint,
                convert_fn,
                build_method_call_fn,
            ) {
                return Ok(expr);
            }

            // Try sequential expression substitution pattern:
            // var1 = expr1; var2 = expr2(var1); return method(var2)
            if let Ok(expr) = sequential_fn(py, &stmts, &arg_names, class_hint) {
                return Ok(expr);
            }

            // Try assignment-based pattern for shadow variable update methods:
            // if cond1: self.field = val1
            // elif cond2: self.field = val2
            // else: self.field = val3
            if let Ok(expr) =
                conditionals::extract_assignment_if(py, &stmts, &arg_names, class_hint, convert_fn)
            {
                return Ok(expr);
            }

            // No explicit return found - method might be more complex
            Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Cannot analyze method: no simple return statement found. \
                 Methods must have a single return expression or assignment pattern.",
            ))
        }

        _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Cannot analyze method: unexpected AST node type '{}'",
            node_type
        ))),
    }
}
