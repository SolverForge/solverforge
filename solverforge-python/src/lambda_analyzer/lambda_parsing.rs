//! Lambda source parsing and extraction.
//!
//! This module handles extracting lambda expressions from Python source code
//! and converting them to Expression trees.

use pyo3::prelude::*;
use pyo3::types::PyList;
use solverforge_core::wasm::Expression;

use super::ast_convert::{convert_ast_to_expression, extract_arg_names};

/// Analyze a Python lambda and convert to an Expression tree.
pub fn analyze_lambda(
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
pub(super) fn analyze_lambda_source(
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
#[allow(clippy::only_used_in_recursion)]
fn extract_lambda_expression(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    param_count: usize,
    class_hint: &str,
) -> PyResult<Option<Expression>> {
    let node_type = node.get_type().name()?.to_string();

    match node_type.as_str() {
        "list" => {
            // Body is a list, find lambda in it
            let list = node.cast::<PyList>()?;
            for item in list.iter() {
                if let Some(expr) = extract_lambda_expression(py, &item, param_count, class_hint)? {
                    return Ok(Some(expr));
                }
            }
            Ok(None)
        }

        "Expr" => {
            // Expression statement wrapper
            let value = node.getattr("value")?;
            extract_lambda_expression(py, &value, param_count, class_hint)
        }

        "Assign" => {
            // Assignment statement - check the value
            let value = node.getattr("value")?;
            extract_lambda_expression(py, &value, param_count, class_hint)
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
                if let Some(expr) = extract_lambda_expression(py, &arg, param_count, class_hint)? {
                    return Ok(Some(expr));
                }
            }
            Ok(None)
        }

        _ => Ok(None),
    }
}
