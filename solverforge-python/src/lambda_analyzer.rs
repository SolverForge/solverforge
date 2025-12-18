//! Lambda analysis for converting Python lambdas to Expression trees.
//!
//! This module provides the infrastructure to analyze Python lambdas at definition time
//! and convert them to `Expression` trees that can be compiled to WASM.
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
//! factory.for_each(Lesson).filter(lambda l: l.room is not None)
//! ```

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

/// Information about a stored lambda.
///
/// This stores the Python callable along with metadata needed for analysis.
pub struct LambdaInfo {
    /// The Python callable (lambda or function).
    pub callable: Py<PyAny>,
    /// Generated unique name for this lambda.
    pub name: String,
    /// Number of parameters the lambda expects.
    pub param_count: usize,
    /// Optional class name hint for type inference.
    pub class_hint: Option<String>,
    /// The analyzed expression (populated after analysis).
    pub expression: Option<Expression>,
}

impl Clone for LambdaInfo {
    fn clone(&self) -> Self {
        Python::attach(|py| Self {
            callable: self.callable.clone_ref(py),
            name: self.name.clone(),
            param_count: self.param_count,
            class_hint: self.class_hint.clone(),
            expression: self.expression.clone(),
        })
    }
}

impl LambdaInfo {
    /// Create a new LambdaInfo from a Python callable.
    ///
    /// This analyzes the lambda immediately and returns an error if the pattern
    /// is not supported.
    pub fn new(py: Python<'_>, callable: Py<PyAny>, prefix: &str) -> PyResult<Self> {
        let name = generate_lambda_name(prefix);
        let param_count = Self::get_param_count(py, &callable)?;

        let mut info = Self {
            callable,
            name,
            param_count,
            class_hint: None,
            expression: None,
        };

        // Analyze the lambda immediately
        let expr = analyze_lambda(py, &info)?;
        info.expression = Some(expr);

        Ok(info)
    }

    /// Create with a class hint for better type inference.
    pub fn with_class_hint(mut self, class_name: impl Into<String>) -> Self {
        self.class_hint = Some(class_name.into());
        self
    }

    /// Get the number of parameters from a Python callable.
    fn get_param_count(py: Python<'_>, callable: &Py<PyAny>) -> PyResult<usize> {
        let inspect = py.import("inspect")?;
        let sig = inspect.call_method1("signature", (callable,))?;
        let params = sig.getattr("parameters")?;
        let len = params.len()?;
        Ok(len)
    }

    /// Convert to a WasmFunction reference.
    pub fn to_wasm_function(&self) -> WasmFunction {
        WasmFunction::new(&self.name)
    }

    /// Get the analyzed expression.
    pub fn get_expression(&self) -> Option<&Expression> {
        self.expression.as_ref()
    }
}

/// Analyze a Python lambda and convert to an Expression tree.
///
/// This function uses Python's AST module to parse the lambda and convert it
/// to a solverforge Expression tree.
///
/// # Errors
///
/// Returns an error with a clear message if the lambda pattern is not supported.
pub fn analyze_lambda(py: Python<'_>, lambda_info: &LambdaInfo) -> PyResult<Expression> {
    let inspect = py.import("inspect")?;

    // Try to get the source code
    let source_result = inspect.call_method1("getsource", (&lambda_info.callable,));

    match source_result {
        Ok(source) => {
            let source_str: String = source.extract()?;
            analyze_lambda_source(py, &source_str, lambda_info)
        }
        Err(_) => {
            // Can't get source - try bytecode analysis as fallback
            analyze_lambda_bytecode(py, lambda_info)
        }
    }
}

/// Analyze lambda from bytecode when source code is unavailable.
///
/// This uses Python's dis module to disassemble the lambda's code object
/// and reconstruct an Expression tree from the bytecode instructions.
fn analyze_lambda_bytecode(py: Python<'_>, lambda_info: &LambdaInfo) -> PyResult<Expression> {
    let callable = lambda_info.callable.bind(py);

    // Use dis module to get instructions - argval contains the resolved values
    let dis = py.import("dis")?;
    let get_instructions = dis.getattr("get_instructions")?;
    let instructions_iter = get_instructions.call1((callable,))?;
    let instructions_list: Vec<Bound<'_, PyAny>> = instructions_iter
        .try_iter()?
        .collect::<Result<Vec<_>, _>>()?;

    // Stack-based evaluation
    let mut stack: Vec<BytecodeValue> = Vec::new();
    let class_name = lambda_info
        .class_hint
        .as_deref()
        .unwrap_or("Unknown")
        .to_string();

    for instr in instructions_list.iter() {
        let opname: String = instr.getattr("opname")?.extract()?;
        let argval = instr.getattr("argval")?;

        match opname.as_str() {
            "RESUME" | "PRECALL" | "PUSH_NULL" | "COPY_FREE_VARS" | "CACHE" => {
                // Skip these opcodes
            }
            "LOAD_FAST" | "LOAD_FAST_CHECK" | "LOAD_FAST_AND_CLEAR" => {
                // Load a local variable (parameter) - argval is the variable name
                // We need to find the parameter index
                let var_name: String = argval.extract()?;
                let code = callable.getattr("__code__")?;
                let varnames: Vec<String> = code.getattr("co_varnames")?.extract()?;
                if let Some(idx) = varnames.iter().position(|n| n == &var_name) {
                    stack.push(BytecodeValue::Param(idx as u32));
                } else {
                    // Variable not in varnames - could be a closure variable
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Cannot analyze lambda: unknown variable '{}'. Lambda parameters must be used directly.",
                        var_name
                    )));
                }
            }
            "LOAD_FAST_LOAD_FAST" => {
                // Python 3.12+ optimization: loads two variables at once
                // argval is a tuple of two variable names like ('a', 'b')
                let var_names: Vec<String> = argval.extract()?;
                let code = callable.getattr("__code__")?;
                let varnames: Vec<String> = code.getattr("co_varnames")?.extract()?;
                for var_name in var_names {
                    if let Some(idx) = varnames.iter().position(|n| n == &var_name) {
                        stack.push(BytecodeValue::Param(idx as u32));
                    } else {
                        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                            "Cannot analyze lambda: unknown variable '{}'. Lambda parameters must be used directly.",
                            var_name
                        )));
                    }
                }
            }
            "LOAD_ATTR" | "LOAD_METHOD" => {
                // Field/method access - argval is the attribute name directly
                let field_name: String = argval.extract()?;
                if let Some(obj) = stack.pop() {
                    stack.push(BytecodeValue::FieldAccess {
                        object: Box::new(obj),
                        class_name: class_name.clone(),
                        field_name,
                    });
                }
            }
            "LOAD_CONST" => {
                // Load a constant - argval is the constant value directly
                if argval.is_none() {
                    stack.push(BytecodeValue::Null);
                } else if let Ok(b) = argval.extract::<bool>() {
                    stack.push(BytecodeValue::Bool(b));
                } else if let Ok(i) = argval.extract::<i64>() {
                    stack.push(BytecodeValue::Int(i));
                }
            }
            "COMPARE_OP" => {
                // Comparison - argval is the operator string (e.g., ">", "==", "!=")
                let op_str: String = argval.extract()?;
                if stack.len() >= 2 {
                    let right = stack.pop().unwrap();
                    let left = stack.pop().unwrap();
                    let result = match op_str.as_str() {
                        "<" => BytecodeValue::Lt(Box::new(left), Box::new(right)),
                        "<=" => BytecodeValue::Le(Box::new(left), Box::new(right)),
                        "==" => BytecodeValue::Eq(Box::new(left), Box::new(right)),
                        "!=" => BytecodeValue::Ne(Box::new(left), Box::new(right)),
                        ">" => BytecodeValue::Gt(Box::new(left), Box::new(right)),
                        ">=" => BytecodeValue::Ge(Box::new(left), Box::new(right)),
                        _ => continue,
                    };
                    stack.push(result);
                }
            }
            "IS_OP" => {
                // is / is not operator - argval is 0 for 'is', 1 for 'is not'
                let invert: i32 = argval.extract()?;
                if stack.len() >= 2 {
                    let right = stack.pop().unwrap();
                    let left = stack.pop().unwrap();
                    let result = if matches!(right, BytecodeValue::Null) {
                        if invert == 0 {
                            BytecodeValue::IsNull(Box::new(left))
                        } else {
                            BytecodeValue::IsNotNull(Box::new(left))
                        }
                    } else if invert == 0 {
                        BytecodeValue::Eq(Box::new(left), Box::new(right))
                    } else {
                        BytecodeValue::Ne(Box::new(left), Box::new(right))
                    };
                    stack.push(result);
                }
            }
            "BINARY_OP" => {
                // Binary arithmetic - argval is the operator index
                let op_idx: i32 = argval.extract()?;
                if stack.len() >= 2 {
                    let right = stack.pop().unwrap();
                    let left = stack.pop().unwrap();
                    let result = match op_idx {
                        0 => BytecodeValue::Add(Box::new(left), Box::new(right)), // +
                        10 => BytecodeValue::Sub(Box::new(left), Box::new(right)), // -
                        5 => BytecodeValue::Mul(Box::new(left), Box::new(right)), // *
                        11 => BytecodeValue::Div(Box::new(left), Box::new(right)), // /
                        _ => continue,
                    };
                    stack.push(result);
                }
            }
            "UNARY_NOT" => {
                // not operator
                if let Some(operand) = stack.pop() {
                    stack.push(BytecodeValue::Not(Box::new(operand)));
                }
            }
            "RETURN_VALUE" | "RETURN_CONST" => {
                // End of function - stack top is our result
                break;
            }
            // Short-circuit boolean operators (and/or)
            // Pattern: expr COPY TO_BOOL POP_JUMP_IF_xxx POP_TOP expr2 RETURN
            "COPY" => {
                // Duplicate top of stack for short-circuit evaluation
                if let Some(top) = stack.last().cloned() {
                    stack.push(top);
                }
            }
            "TO_BOOL" => {
                // TO_BOOL converts top to bool for jump decision
                // In our analysis, we just leave the original value - it's for control flow
                // Don't modify stack
            }
            "POP_TOP" => {
                // Pop and discard - but don't pop PendingAnd/PendingOr markers
                if let Some(top) = stack.last() {
                    if !matches!(
                        top,
                        BytecodeValue::PendingAnd(_) | BytecodeValue::PendingOr(_)
                    ) {
                        stack.pop();
                    }
                }
            }
            "POP_JUMP_IF_FALSE" => {
                // This is part of AND short-circuit: if false, jump to end
                // At this point we have: [original, copy_for_bool_check]
                // We pop both (one for TO_BOOL decision, one to mark as AND)
                // Then push PendingAnd with the original value
                if stack.len() >= 2 {
                    stack.pop(); // Pop the bool-check copy
                    if let Some(left) = stack.pop() {
                        stack.push(BytecodeValue::PendingAnd(Box::new(left)));
                    }
                } else if let Some(left) = stack.pop() {
                    stack.push(BytecodeValue::PendingAnd(Box::new(left)));
                }
            }
            "POP_JUMP_IF_TRUE" => {
                // This is part of OR short-circuit: if true, jump to end
                if stack.len() >= 2 {
                    stack.pop(); // Pop the bool-check copy
                    if let Some(left) = stack.pop() {
                        stack.push(BytecodeValue::PendingOr(Box::new(left)));
                    }
                } else if let Some(left) = stack.pop() {
                    stack.push(BytecodeValue::PendingOr(Box::new(left)));
                }
            }
            // Reject unsupported opcodes that reference external state
            "LOAD_GLOBAL" | "LOAD_DEREF" | "LOAD_CLOSURE" | "LOAD_NAME" => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Cannot analyze lambda: references external variable. \
                     Use only lambda parameters and literals. Found opcode: {}",
                    opname
                )));
            }
            _ => {
                // Unknown opcode - may cause issues
            }
        }
    }

    // Check for pending AND/OR that need to be completed
    // If we have PendingAnd/PendingOr followed by a value, combine them
    if stack.len() >= 2 {
        let right = stack.pop().unwrap();
        let pending = stack.pop().unwrap();
        match pending {
            BytecodeValue::PendingAnd(left) => {
                stack.push(BytecodeValue::And(left, Box::new(right)));
            }
            BytecodeValue::PendingOr(left) => {
                stack.push(BytecodeValue::Or(left, Box::new(right)));
            }
            _ => {
                // Put them back if not a pending boolean op
                stack.push(pending);
                stack.push(right);
            }
        }
    }

    // Convert top of stack to Expression
    if let Some(top) = stack.pop() {
        bytecode_value_to_expression(top)
    } else {
        Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Cannot analyze lambda: bytecode analysis failed. \
             Use a simple lambda like `lambda x: x.field`.",
        ))
    }
}

/// Intermediate representation for bytecode analysis.
#[derive(Debug, Clone)]
enum BytecodeValue {
    Param(u32),
    Null,
    Bool(bool),
    Int(i64),
    FieldAccess {
        object: Box<BytecodeValue>,
        class_name: String,
        field_name: String,
    },
    Eq(Box<BytecodeValue>, Box<BytecodeValue>),
    Ne(Box<BytecodeValue>, Box<BytecodeValue>),
    Lt(Box<BytecodeValue>, Box<BytecodeValue>),
    Le(Box<BytecodeValue>, Box<BytecodeValue>),
    Gt(Box<BytecodeValue>, Box<BytecodeValue>),
    Ge(Box<BytecodeValue>, Box<BytecodeValue>),
    IsNull(Box<BytecodeValue>),
    IsNotNull(Box<BytecodeValue>),
    Add(Box<BytecodeValue>, Box<BytecodeValue>),
    Sub(Box<BytecodeValue>, Box<BytecodeValue>),
    Mul(Box<BytecodeValue>, Box<BytecodeValue>),
    Div(Box<BytecodeValue>, Box<BytecodeValue>),
    Not(Box<BytecodeValue>),
    And(Box<BytecodeValue>, Box<BytecodeValue>),
    Or(Box<BytecodeValue>, Box<BytecodeValue>),
    // Temporary markers for short-circuit evaluation pattern
    PendingAnd(Box<BytecodeValue>),
    PendingOr(Box<BytecodeValue>),
}

/// Convert BytecodeValue to Expression.
fn bytecode_value_to_expression(value: BytecodeValue) -> PyResult<Expression> {
    match value {
        BytecodeValue::Param(index) => Ok(Expression::Param { index }),
        BytecodeValue::Null => Ok(Expression::Null),
        BytecodeValue::Bool(v) => Ok(Expression::BoolLiteral { value: v }),
        BytecodeValue::Int(v) => Ok(Expression::IntLiteral { value: v }),
        BytecodeValue::FieldAccess {
            object,
            class_name,
            field_name,
        } => Ok(Expression::FieldAccess {
            object: Box::new(bytecode_value_to_expression(*object)?),
            class_name,
            field_name,
        }),
        BytecodeValue::Eq(l, r) => Ok(Expression::Eq {
            left: Box::new(bytecode_value_to_expression(*l)?),
            right: Box::new(bytecode_value_to_expression(*r)?),
        }),
        BytecodeValue::Ne(l, r) => Ok(Expression::Ne {
            left: Box::new(bytecode_value_to_expression(*l)?),
            right: Box::new(bytecode_value_to_expression(*r)?),
        }),
        BytecodeValue::Lt(l, r) => Ok(Expression::Lt {
            left: Box::new(bytecode_value_to_expression(*l)?),
            right: Box::new(bytecode_value_to_expression(*r)?),
        }),
        BytecodeValue::Le(l, r) => Ok(Expression::Le {
            left: Box::new(bytecode_value_to_expression(*l)?),
            right: Box::new(bytecode_value_to_expression(*r)?),
        }),
        BytecodeValue::Gt(l, r) => Ok(Expression::Gt {
            left: Box::new(bytecode_value_to_expression(*l)?),
            right: Box::new(bytecode_value_to_expression(*r)?),
        }),
        BytecodeValue::Ge(l, r) => Ok(Expression::Ge {
            left: Box::new(bytecode_value_to_expression(*l)?),
            right: Box::new(bytecode_value_to_expression(*r)?),
        }),
        BytecodeValue::IsNull(operand) => Ok(Expression::IsNull {
            operand: Box::new(bytecode_value_to_expression(*operand)?),
        }),
        BytecodeValue::IsNotNull(operand) => Ok(Expression::IsNotNull {
            operand: Box::new(bytecode_value_to_expression(*operand)?),
        }),
        BytecodeValue::Add(l, r) => Ok(Expression::Add {
            left: Box::new(bytecode_value_to_expression(*l)?),
            right: Box::new(bytecode_value_to_expression(*r)?),
        }),
        BytecodeValue::Sub(l, r) => Ok(Expression::Sub {
            left: Box::new(bytecode_value_to_expression(*l)?),
            right: Box::new(bytecode_value_to_expression(*r)?),
        }),
        BytecodeValue::Mul(l, r) => Ok(Expression::Mul {
            left: Box::new(bytecode_value_to_expression(*l)?),
            right: Box::new(bytecode_value_to_expression(*r)?),
        }),
        BytecodeValue::Div(l, r) => Ok(Expression::Div {
            left: Box::new(bytecode_value_to_expression(*l)?),
            right: Box::new(bytecode_value_to_expression(*r)?),
        }),
        BytecodeValue::Not(operand) => Ok(Expression::Not {
            operand: Box::new(bytecode_value_to_expression(*operand)?),
        }),
        BytecodeValue::And(l, r) => Ok(Expression::And {
            left: Box::new(bytecode_value_to_expression(*l)?),
            right: Box::new(bytecode_value_to_expression(*r)?),
        }),
        BytecodeValue::Or(l, r) => Ok(Expression::Or {
            left: Box::new(bytecode_value_to_expression(*l)?),
            right: Box::new(bytecode_value_to_expression(*r)?),
        }),
        BytecodeValue::PendingAnd(_) | BytecodeValue::PendingOr(_) => {
            // These should have been resolved - incomplete boolean expression
            Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Cannot analyze lambda: incomplete boolean expression.",
            ))
        }
    }
}

/// Analyze lambda from source code.
fn analyze_lambda_source(
    py: Python<'_>,
    source: &str,
    lambda_info: &LambdaInfo,
) -> PyResult<Expression> {
    let ast = py.import("ast")?;

    // Try to extract just the lambda expression from the source
    // Source might be like ".filter(lambda x: x.field)" which isn't valid Python
    let lambda_source = extract_lambda_from_source(source);

    // Try parsing the extracted lambda source
    let parse_result = ast.call_method1("parse", (&lambda_source,));

    let tree = match parse_result {
        Ok(t) => t,
        Err(_) => {
            // If parsing fails, try bytecode analysis
            return analyze_lambda_bytecode(py, lambda_info);
        }
    };

    // Walk the AST to find the lambda expression
    let body = tree.getattr("body")?;

    // Extract lambda node and convert to Expression
    match extract_lambda_expression(py, &body, lambda_info)? {
        Some(expr) => Ok(expr),
        None => {
            // Fallback to bytecode analysis if AST extraction fails
            analyze_lambda_bytecode(py, lambda_info)
        }
    }
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
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => {
                    if depth == 0 {
                        // Found closing paren that ends the lambda
                        end_idx = i;
                        break;
                    }
                    depth -= 1;
                }
                ',' if depth == 0 => {
                    // Comma at depth 0 ends the lambda argument
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
    lambda_info: &LambdaInfo,
) -> PyResult<Option<Expression>> {
    let node_type = node.get_type().name()?.to_string();

    match node_type.as_str() {
        "list" => {
            // Body is a list, find lambda in it
            let list = node.cast::<PyList>()?;
            for item in list.iter() {
                if let Some(expr) = extract_lambda_expression(py, &item, lambda_info)? {
                    return Ok(Some(expr));
                }
            }
            Ok(None)
        }

        "Expr" => {
            // Expression statement wrapper
            let value = node.getattr("value")?;
            extract_lambda_expression(py, &value, lambda_info)
        }

        "Assign" => {
            // Assignment statement - check the value
            let value = node.getattr("value")?;
            extract_lambda_expression(py, &value, lambda_info)
        }

        "Lambda" => {
            // Found the lambda - analyze its body
            let body = node.getattr("body")?;
            let args = node.getattr("args")?;
            let arg_names = extract_arg_names(py, &args)?;

            convert_ast_to_expression(py, &body, &arg_names, lambda_info)
        }

        "Call" => {
            // Function call - might wrap a lambda
            let args_node = node.getattr("args")?;
            let args_list = args_node.cast::<PyList>()?;

            for arg in args_list.iter() {
                if let Some(expr) = extract_lambda_expression(py, &arg, lambda_info)? {
                    return Ok(Some(expr));
                }
            }
            Ok(None)
        }

        _ => Ok(None),
    }
}

/// Extract argument names from Python AST arguments node.
fn extract_arg_names(_py: Python<'_>, args: &Bound<'_, PyAny>) -> PyResult<Vec<String>> {
    let arg_list = args.getattr("args")?;
    let list = arg_list.cast::<PyList>()?;

    let mut names = Vec::new();
    for arg in list.iter() {
        let arg_name: String = arg.getattr("arg")?.extract()?;
        names.push(arg_name);
    }

    Ok(names)
}

/// Convert Python AST node to Expression tree.
fn convert_ast_to_expression(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    arg_names: &[String],
    lambda_info: &LambdaInfo,
) -> PyResult<Option<Expression>> {
    let node_type = node.get_type().name()?.to_string();
    let class_name = lambda_info
        .class_hint
        .as_deref()
        .unwrap_or("Unknown")
        .to_string();

    match node_type.as_str() {
        "Attribute" => {
            // Field access: x.field
            let value = node.getattr("value")?;
            let attr: String = node.getattr("attr")?.extract()?;

            if let Some(base_expr) = convert_ast_to_expression(py, &value, arg_names, lambda_info)?
            {
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
            convert_compare_to_expression(py, node, arg_names, lambda_info)
        }

        "BoolOp" => {
            // Boolean operation: and, or
            convert_boolop_to_expression(py, node, arg_names, lambda_info)
        }

        "UnaryOp" => {
            // Unary operation: not
            convert_unaryop_to_expression(py, node, arg_names, lambda_info)
        }

        "BinOp" => {
            // Binary operation: +, -, *, /
            convert_binop_to_expression(py, node, arg_names, lambda_info)
        }

        "Constant" | "Num" | "NameConstant" => {
            // Literal value
            convert_constant_to_expression(node)
        }

        _ => Ok(None),
    }
}

/// Convert Python Compare AST node to Expression.
fn convert_compare_to_expression(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    arg_names: &[String],
    lambda_info: &LambdaInfo,
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

    let left_expr = convert_ast_to_expression(py, &left, arg_names, lambda_info)?;
    let right_expr = convert_ast_to_expression(py, &comparators[0], arg_names, lambda_info)?;

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
fn convert_boolop_to_expression(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    arg_names: &[String],
    lambda_info: &LambdaInfo,
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
        if let Some(expr) = convert_ast_to_expression(py, val, arg_names, lambda_info)? {
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
fn convert_unaryop_to_expression(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    arg_names: &[String],
    lambda_info: &LambdaInfo,
) -> PyResult<Option<Expression>> {
    let op = node.getattr("op")?;
    let operand = node.getattr("operand")?;

    let op_type = op.get_type().name()?.to_string();

    if let Some(operand_expr) = convert_ast_to_expression(py, &operand, arg_names, lambda_info)? {
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
fn convert_binop_to_expression(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    arg_names: &[String],
    lambda_info: &LambdaInfo,
) -> PyResult<Option<Expression>> {
    let op = node.getattr("op")?;
    let left = node.getattr("left")?;
    let right = node.getattr("right")?;

    let left_expr = convert_ast_to_expression(py, &left, arg_names, lambda_info)?;
    let right_expr = convert_ast_to_expression(py, &right, arg_names, lambda_info)?;

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
fn convert_constant_to_expression(node: &Bound<'_, PyAny>) -> PyResult<Option<Expression>> {
    let node_type = node.get_type().name()?.to_string();

    match node_type.as_str() {
        "Constant" => {
            let value = node.getattr("value")?;

            if value.is_none() {
                Ok(Some(Expression::Null))
            } else if let Ok(b) = value.extract::<bool>() {
                Ok(Some(Expression::BoolLiteral { value: b }))
            } else if let Ok(i) = value.extract::<i64>() {
                Ok(Some(Expression::IntLiteral { value: i }))
            } else {
                Ok(None)
            }
        }
        "NameConstant" => {
            // Python 3.7 style: None, True, False
            let value = node.getattr("value")?;
            if value.is_none() {
                Ok(Some(Expression::Null))
            } else if let Ok(b) = value.extract::<bool>() {
                Ok(Some(Expression::BoolLiteral { value: b }))
            } else {
                Ok(None)
            }
        }
        "Num" => {
            // Python 3.7 style numbers
            let n = node.getattr("n")?;
            if let Ok(i) = n.extract::<i64>() {
                Ok(Some(Expression::IntLiteral { value: i }))
            } else {
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyo3::types::PyDict;

    fn init_python() {
        pyo3::prepare_freethreaded_python();
    }

    #[test]
    fn test_generate_lambda_name_unique() {
        let name1 = generate_lambda_name("test");
        let name2 = generate_lambda_name("test");
        assert_ne!(name1, name2);
        assert!(name1.starts_with("test_"));
        assert!(name2.starts_with("test_"));
    }

    #[test]
    fn test_lambda_info_param_count() {
        init_python();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(c"f = lambda x: x", None, Some(&locals)).unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            let count = LambdaInfo::get_param_count(py, &func.unbind()).unwrap();
            assert_eq!(count, 1);
        });
    }

    #[test]
    fn test_lambda_info_param_count_two() {
        init_python();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(c"f = lambda a, b: a", None, Some(&locals)).unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            let count = LambdaInfo::get_param_count(py, &func.unbind()).unwrap();
            assert_eq!(count, 2);
        });
    }

    #[test]
    fn test_analyze_simple_field_access() {
        init_python();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(c"f = lambda x: x.timeslot", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            let info = LambdaInfo {
                callable: func.unbind(),
                name: "test".to_string(),
                param_count: 1,
                class_hint: Some("Lesson".to_string()),
                expression: None,
            };

            let result = analyze_lambda(py, &info).unwrap();

            match result {
                Expression::FieldAccess {
                    field_name,
                    class_name,
                    ..
                } => {
                    assert_eq!(field_name, "timeslot");
                    assert_eq!(class_name, "Lesson");
                }
                _ => panic!("Expected FieldAccess, got {:?}", result),
            }
        });
    }

    #[test]
    fn test_analyze_is_not_none() {
        init_python();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(c"f = lambda x: x.room is not None", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            let info = LambdaInfo {
                callable: func.unbind(),
                name: "test".to_string(),
                param_count: 1,
                class_hint: Some("Lesson".to_string()),
                expression: None,
            };

            let result = analyze_lambda(py, &info).unwrap();

            match result {
                Expression::IsNotNull { operand } => match *operand {
                    Expression::FieldAccess { field_name, .. } => {
                        assert_eq!(field_name, "room");
                    }
                    _ => panic!("Expected FieldAccess inside IsNotNull"),
                },
                _ => panic!("Expected IsNotNull, got {:?}", result),
            }
        });
    }

    #[test]
    fn test_analyze_is_none() {
        init_python();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(c"f = lambda x: x.room is None", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            let info = LambdaInfo {
                callable: func.unbind(),
                name: "test".to_string(),
                param_count: 1,
                class_hint: None,
                expression: None,
            };

            let result = analyze_lambda(py, &info).unwrap();

            assert!(matches!(result, Expression::IsNull { .. }));
        });
    }

    #[test]
    fn test_analyze_comparison_gt() {
        init_python();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(c"f = lambda x: x.count > 5", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            let info = LambdaInfo {
                callable: func.unbind(),
                name: "test".to_string(),
                param_count: 1,
                class_hint: None,
                expression: None,
            };

            let result = analyze_lambda(py, &info).unwrap();

            match result {
                Expression::Gt { left, right } => {
                    assert!(matches!(*left, Expression::FieldAccess { .. }));
                    assert!(matches!(*right, Expression::IntLiteral { value: 5 }));
                }
                _ => panic!("Expected Gt, got {:?}", result),
            }
        });
    }

    #[test]
    fn test_analyze_comparison_eq() {
        init_python();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(c"f = lambda x: x.status == 1", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            let info = LambdaInfo {
                callable: func.unbind(),
                name: "test".to_string(),
                param_count: 1,
                class_hint: None,
                expression: None,
            };

            let result = analyze_lambda(py, &info).unwrap();

            assert!(matches!(result, Expression::Eq { .. }));
        });
    }

    #[test]
    fn test_analyze_and_expression() {
        init_python();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(
                c"f = lambda x: x.room is not None and x.timeslot is not None",
                None,
                Some(&locals),
            )
            .unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            let info = LambdaInfo {
                callable: func.unbind(),
                name: "test".to_string(),
                param_count: 1,
                class_hint: None,
                expression: None,
            };

            let result = analyze_lambda(py, &info).unwrap();

            assert!(matches!(result, Expression::And { .. }));
        });
    }

    #[test]
    fn test_analyze_or_expression() {
        init_python();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(c"f = lambda x: x.a > 0 or x.b > 0", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            let info = LambdaInfo {
                callable: func.unbind(),
                name: "test".to_string(),
                param_count: 1,
                class_hint: None,
                expression: None,
            };

            let result = analyze_lambda(py, &info).unwrap();

            assert!(matches!(result, Expression::Or { .. }));
        });
    }

    #[test]
    fn test_analyze_not_expression() {
        init_python();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(c"f = lambda x: not x.active", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            let info = LambdaInfo {
                callable: func.unbind(),
                name: "test".to_string(),
                param_count: 1,
                class_hint: None,
                expression: None,
            };

            let result = analyze_lambda(py, &info).unwrap();

            assert!(matches!(result, Expression::Not { .. }));
        });
    }

    #[test]
    fn test_analyze_arithmetic_add() {
        init_python();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(c"f = lambda x: x.value + 10", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            let info = LambdaInfo {
                callable: func.unbind(),
                name: "test".to_string(),
                param_count: 1,
                class_hint: None,
                expression: None,
            };

            let result = analyze_lambda(py, &info).unwrap();

            assert!(matches!(result, Expression::Add { .. }));
        });
    }

    #[test]
    fn test_analyze_arithmetic_sub() {
        init_python();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(c"f = lambda x: x.value - 5", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            let info = LambdaInfo {
                callable: func.unbind(),
                name: "test".to_string(),
                param_count: 1,
                class_hint: None,
                expression: None,
            };

            let result = analyze_lambda(py, &info).unwrap();

            assert!(matches!(result, Expression::Sub { .. }));
        });
    }

    #[test]
    fn test_analyze_arithmetic_mul() {
        init_python();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(c"f = lambda x: x.value * 2", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            let info = LambdaInfo {
                callable: func.unbind(),
                name: "test".to_string(),
                param_count: 1,
                class_hint: None,
                expression: None,
            };

            let result = analyze_lambda(py, &info).unwrap();

            assert!(matches!(result, Expression::Mul { .. }));
        });
    }

    #[test]
    fn test_analyze_arithmetic_div() {
        init_python();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(c"f = lambda x: x.value / 2", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            let info = LambdaInfo {
                callable: func.unbind(),
                name: "test".to_string(),
                param_count: 1,
                class_hint: None,
                expression: None,
            };

            let result = analyze_lambda(py, &info).unwrap();

            assert!(matches!(result, Expression::Div { .. }));
        });
    }

    #[test]
    fn test_analyze_bi_lambda() {
        init_python();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(c"f = lambda a, b: a.room == b.room", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            let info = LambdaInfo {
                callable: func.unbind(),
                name: "test".to_string(),
                param_count: 2,
                class_hint: None,
                expression: None,
            };

            let result = analyze_lambda(py, &info).unwrap();

            match result {
                Expression::Eq { left, right } => {
                    // Verify both sides are field accesses from different params
                    match (*left, *right) {
                        (
                            Expression::FieldAccess {
                                object: left_obj, ..
                            },
                            Expression::FieldAccess {
                                object: right_obj, ..
                            },
                        ) => {
                            assert!(matches!(*left_obj, Expression::Param { index: 0 }));
                            assert!(matches!(*right_obj, Expression::Param { index: 1 }));
                        }
                        _ => panic!("Expected field accesses"),
                    }
                }
                _ => panic!("Expected Eq expression"),
            }
        });
    }

    #[test]
    fn test_analyze_bi_lambda_direct_param_add() {
        // Tests LOAD_FAST_LOAD_FAST bytecode (Python 3.12+)
        // This is used by compose() combiner lambdas like: lambda a, b: a + b
        init_python();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(c"f = lambda a, b: a + b", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            let info = LambdaInfo {
                callable: func.unbind(),
                name: "test".to_string(),
                param_count: 2,
                class_hint: None,
                expression: None,
            };

            let result = analyze_lambda(py, &info).unwrap();

            match result {
                Expression::Add { left, right } => {
                    assert!(matches!(*left, Expression::Param { index: 0 }));
                    assert!(matches!(*right, Expression::Param { index: 1 }));
                }
                _ => panic!("Expected Add expression, got {:?}", result),
            }
        });
    }

    #[test]
    fn test_analyze_tri_lambda_arithmetic() {
        // Tests three-parameter lambda with arithmetic
        init_python();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(c"f = lambda a, b, c: a + b + c", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            let info = LambdaInfo {
                callable: func.unbind(),
                name: "test".to_string(),
                param_count: 3,
                class_hint: None,
                expression: None,
            };

            let result = analyze_lambda(py, &info).unwrap();

            // Should be Add(Add(a, b), c)
            match result {
                Expression::Add { left, right } => {
                    // right should be Param 2 (c)
                    assert!(matches!(*right, Expression::Param { index: 2 }));
                    // left should be Add(a, b)
                    match *left {
                        Expression::Add {
                            left: l2,
                            right: r2,
                        } => {
                            assert!(matches!(*l2, Expression::Param { index: 0 }));
                            assert!(matches!(*r2, Expression::Param { index: 1 }));
                        }
                        _ => panic!("Expected nested Add"),
                    }
                }
                _ => panic!("Expected Add expression, got {:?}", result),
            }
        });
    }

    #[test]
    fn test_analyze_nested_field_access() {
        init_python();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(c"f = lambda x: x.employee.name", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            let info = LambdaInfo {
                callable: func.unbind(),
                name: "test".to_string(),
                param_count: 1,
                class_hint: None,
                expression: None,
            };

            let result = analyze_lambda(py, &info).unwrap();

            match result {
                Expression::FieldAccess {
                    field_name, object, ..
                } => {
                    assert_eq!(field_name, "name");
                    // The object should be another FieldAccess
                    match *object {
                        Expression::FieldAccess { field_name, .. } => {
                            assert_eq!(field_name, "employee");
                        }
                        _ => panic!("Expected nested FieldAccess"),
                    }
                }
                _ => panic!("Expected FieldAccess"),
            }
        });
    }

    #[test]
    fn test_lambda_info_new_analyzes_immediately() {
        init_python();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(c"f = lambda x: x.field", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            let info = LambdaInfo::new(py, func.unbind(), "test").unwrap();

            // Expression should be populated
            assert!(info.expression.is_some());
            assert!(matches!(
                info.expression.unwrap(),
                Expression::FieldAccess { .. }
            ));
        });
    }

    #[test]
    fn test_lambda_info_to_wasm_function() {
        init_python();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(c"f = lambda x: x.field", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            let info = LambdaInfo::new(py, func.unbind(), "equal_map").unwrap();
            let wasm_func = info.to_wasm_function();

            assert!(wasm_func.name().starts_with("equal_map_"));
        });
    }

    #[test]
    fn test_extract_lambda_from_filter_call() {
        let source = ".filter(lambda vehicle: vehicle.calculate_total_demand() > vehicle.capacity)";
        let result = extract_lambda_from_source(source);
        assert_eq!(
            result,
            "_ = lambda vehicle: vehicle.calculate_total_demand() > vehicle.capacity"
        );
    }

    #[test]
    fn test_extract_lambda_from_penalize_call() {
        let source =
            "        .penalize(HardSoftScore.ONE_HARD, lambda vehicle: vehicle.demand - 10)";
        let result = extract_lambda_from_source(source);
        assert_eq!(result, "_ = lambda vehicle: vehicle.demand - 10");
    }

    #[test]
    fn test_extract_lambda_simple() {
        let source = "lambda x: x.field";
        let result = extract_lambda_from_source(source);
        assert_eq!(result, "_ = lambda x: x.field");
    }

    #[test]
    fn test_extract_lambda_with_nested_parens() {
        let source = ".filter(lambda x: (x.a + x.b) > 0)";
        let result = extract_lambda_from_source(source);
        assert_eq!(result, "_ = lambda x: (x.a + x.b) > 0");
    }

    #[test]
    fn test_extract_lambda_second_arg() {
        let source = ".penalize(Score.ONE, lambda x: x.value)";
        let result = extract_lambda_from_source(source);
        assert_eq!(result, "_ = lambda x: x.value");
    }

    #[test]
    fn test_extract_lambda_no_lambda() {
        let source = "some_other_code()";
        let result = extract_lambda_from_source(source);
        assert_eq!(result, "some_other_code()");
    }
}
