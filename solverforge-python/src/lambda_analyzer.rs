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
//! factory.for_each("Lesson").filter(lambda l: l.room is not None)
//! ```

use pyo3::prelude::*;
use pyo3::types::PyList;
use solverforge_core::constraints::WasmFunction;
use solverforge_core::wasm::Expression;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

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
#[derive(Clone)]
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

/// Get the number of parameters from a Python callable.
fn get_param_count(py: Python<'_>, callable: &Py<PyAny>) -> PyResult<usize> {
    let inspect = py.import("inspect")?;
    let sig = inspect.call_method1("signature", (callable,))?;
    let params = sig.getattr("parameters")?;
    let len = params.len()?;
    Ok(len)
}

// ============================================================================
// Method Introspection Helpers
// ============================================================================

/// Global registry for domain classes that can be introspected.
///
/// This stores references to Python classes decorated with @planning_entity
/// or @planning_solution, enabling method body analysis for inlining.
static CLASS_REGISTRY: RwLock<Option<HashMap<String, Py<PyAny>>>> = RwLock::new(None);

/// Register a Python class for method introspection.
///
/// Called by @planning_entity and @planning_solution decorators.
pub fn register_class(py: Python<'_>, class_name: &str, class: &Bound<'_, PyAny>) {
    let mut registry = CLASS_REGISTRY.write().unwrap();
    if registry.is_none() {
        *registry = Some(HashMap::new());
    }
    if let Some(ref mut map) = *registry {
        map.insert(class_name.to_string(), class.clone().unbind());
        log::debug!("Registered class '{}' for method introspection", class_name);
    }
    drop(registry);

    // Also store on the class itself for access during solving
    let _ = class.setattr("__solverforge_class_name__", class_name);
    let _ = py; // suppress unused warning
}

/// Look up a method from a registered domain class.
///
/// Returns the method object if found, or None if the class/method doesn't exist.
///
/// # Arguments
/// * `py` - Python interpreter
/// * `class_name` - Name of the class (e.g., "Vehicle")
/// * `method_name` - Name of the method (e.g., "calculate_total_demand")
///
/// # Returns
/// * `Some(Py<PyAny>)` - The method callable if found
/// * `None` - If class or method not found
pub fn get_method_from_class(
    py: Python<'_>,
    class_name: &str,
    method_name: &str,
) -> Option<Py<PyAny>> {
    let registry = CLASS_REGISTRY.read().unwrap();

    if let Some(ref map) = *registry {
        if let Some(class) = map.get(class_name) {
            let class_bound = class.bind(py);

            // Try to get the method from the class
            if let Ok(method) = class_bound.getattr(method_name) {
                // Check if it's actually a method/function (not a class attribute)
                let inspect = py.import("inspect").ok()?;
                let is_method = inspect
                    .call_method1("isfunction", (&method,))
                    .ok()?
                    .extract::<bool>()
                    .ok()?;
                let is_method_descriptor = inspect
                    .call_method1("ismethod", (&method,))
                    .ok()?
                    .extract::<bool>()
                    .ok()?;

                if is_method || is_method_descriptor {
                    log::debug!("Found method '{}' on class '{}'", method_name, class_name);
                    return Some(method.unbind());
                }
            }
        }
    }

    log::debug!(
        "Method '{}' not found on class '{}' in registry",
        method_name,
        class_name
    );
    None
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
///
/// # Returns
/// An Expression tree representing the method body, where:
/// - `Param { index: 0 }` represents `self`
/// - Other params are indexed 1, 2, etc.
///
/// # Errors
/// Returns an error if the method body uses unsupported patterns.
pub fn analyze_method_body(
    py: Python<'_>,
    method: &Py<PyAny>,
    class_hint: &str,
) -> PyResult<Expression> {
    let param_count = get_param_count(py, method)?;

    // Try source analysis first, fall back to bytecode
    let inspect = py.import("inspect")?;
    let source_result = inspect.call_method1("getsource", (method,));

    match source_result {
        Ok(source) => {
            let source_str: String = source.extract()?;
            // Methods have their body after "def method_name(self, ...):"
            // We need to extract and analyze the return expression
            analyze_method_source(py, &source_str, method, param_count, class_hint)
        }
        Err(_) => {
            // Bytecode analysis works the same way
            analyze_lambda_bytecode(py, method, param_count, class_hint)
        }
    }
}

/// Analyze method from source code.
///
/// Extracts the return expression from a method definition and analyzes it.
fn analyze_method_source(
    py: Python<'_>,
    source: &str,
    method: &Py<PyAny>,
    param_count: usize,
    class_hint: &str,
) -> PyResult<Expression> {
    let ast = py.import("ast")?;

    // Parse the method source
    // We need to handle indentation - dedent the source first
    let textwrap = py.import("textwrap")?;
    let dedented: String = textwrap.call_method1("dedent", (source,))?.extract()?;

    let parse_result = ast.call_method1("parse", (&dedented,));

    let tree = match parse_result {
        Ok(t) => t,
        Err(_) => {
            // If parsing fails, try bytecode analysis
            return analyze_lambda_bytecode(py, method, param_count, class_hint);
        }
    };

    // Find the FunctionDef node and extract its return expression
    let body = tree.getattr("body")?;
    extract_method_return_expression(py, &body, param_count, class_hint)
}

/// Extract the return expression from a method's AST.
fn extract_method_return_expression(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    _param_count: usize,
    class_hint: &str,
) -> PyResult<Expression> {
    let node_type = node.get_type().name()?.to_string();

    match node_type.as_str() {
        "list" => {
            // Body is a list, find FunctionDef
            let list = node.cast::<PyList>()?;
            for item in list.iter() {
                let item_type = item.get_type().name()?.to_string();
                if item_type == "FunctionDef" || item_type == "AsyncFunctionDef" {
                    return extract_method_return_expression(py, &item, _param_count, class_hint);
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

            for stmt in body_list.iter() {
                let stmt_type = stmt.get_type().name()?.to_string();
                if stmt_type == "Return" {
                    let value = stmt.getattr("value")?;
                    if !value.is_none() {
                        if let Some(expr) =
                            convert_ast_to_expression(py, &value, &arg_names, class_hint)?
                        {
                            return Ok(expr);
                        }
                    }
                }
            }

            // No explicit return found - method might be more complex
            Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Cannot analyze method: no simple return statement found. \
                 Methods must have a single return expression.",
            ))
        }

        _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Cannot analyze method: unexpected AST node type '{}'",
            node_type
        ))),
    }
}

/// Clear the class registry (for testing).
#[cfg(test)]
pub fn clear_class_registry() {
    let mut registry = CLASS_REGISTRY.write().unwrap();
    *registry = None;
}

// ============================================================================
// Expression Substitution
// ============================================================================

/// Substitute a parameter in an expression tree with another expression.
///
/// This is used for method inlining: when we analyze a method body, `self`
/// becomes `Param { index: 0 }`. To inline the method call, we substitute
/// that parameter with the actual calling object's expression.
///
/// # Arguments
/// * `expr` - The expression to transform
/// * `from_index` - The parameter index to replace (usually 0 for `self`)
/// * `substitute` - The expression to replace it with
///
/// # Example
/// ```text
/// // Method: def get_total(self): return self.demand + self.capacity
/// // Analyzed as: Add(FieldAccess(Param(0), "demand"), FieldAccess(Param(0), "capacity"))
/// //
/// // Lambda: lambda v: v.get_total() > 100
/// // After inlining with substitute_param(method_expr, 0, Param(0)):
/// // Gt(Add(FieldAccess(Param(0), "demand"), FieldAccess(Param(0), "capacity")), IntLiteral(100))
/// ```
pub fn substitute_param(expr: Expression, from_index: u32, substitute: &Expression) -> Expression {
    match expr {
        Expression::Param { index } if index == from_index => substitute.clone(),
        Expression::Param { index } => Expression::Param { index },

        // Recursively substitute in compound expressions
        Expression::FieldAccess {
            object,
            class_name,
            field_name,
        } => Expression::FieldAccess {
            object: Box::new(substitute_param(*object, from_index, substitute)),
            class_name,
            field_name,
        },

        Expression::Eq { left, right } => Expression::Eq {
            left: Box::new(substitute_param(*left, from_index, substitute)),
            right: Box::new(substitute_param(*right, from_index, substitute)),
        },
        Expression::Ne { left, right } => Expression::Ne {
            left: Box::new(substitute_param(*left, from_index, substitute)),
            right: Box::new(substitute_param(*right, from_index, substitute)),
        },
        Expression::Lt { left, right } => Expression::Lt {
            left: Box::new(substitute_param(*left, from_index, substitute)),
            right: Box::new(substitute_param(*right, from_index, substitute)),
        },
        Expression::Le { left, right } => Expression::Le {
            left: Box::new(substitute_param(*left, from_index, substitute)),
            right: Box::new(substitute_param(*right, from_index, substitute)),
        },
        Expression::Gt { left, right } => Expression::Gt {
            left: Box::new(substitute_param(*left, from_index, substitute)),
            right: Box::new(substitute_param(*right, from_index, substitute)),
        },
        Expression::Ge { left, right } => Expression::Ge {
            left: Box::new(substitute_param(*left, from_index, substitute)),
            right: Box::new(substitute_param(*right, from_index, substitute)),
        },

        Expression::Add { left, right } => Expression::Add {
            left: Box::new(substitute_param(*left, from_index, substitute)),
            right: Box::new(substitute_param(*right, from_index, substitute)),
        },
        Expression::Sub { left, right } => Expression::Sub {
            left: Box::new(substitute_param(*left, from_index, substitute)),
            right: Box::new(substitute_param(*right, from_index, substitute)),
        },
        Expression::Mul { left, right } => Expression::Mul {
            left: Box::new(substitute_param(*left, from_index, substitute)),
            right: Box::new(substitute_param(*right, from_index, substitute)),
        },
        Expression::Div { left, right } => Expression::Div {
            left: Box::new(substitute_param(*left, from_index, substitute)),
            right: Box::new(substitute_param(*right, from_index, substitute)),
        },

        Expression::And { left, right } => Expression::And {
            left: Box::new(substitute_param(*left, from_index, substitute)),
            right: Box::new(substitute_param(*right, from_index, substitute)),
        },
        Expression::Or { left, right } => Expression::Or {
            left: Box::new(substitute_param(*left, from_index, substitute)),
            right: Box::new(substitute_param(*right, from_index, substitute)),
        },
        Expression::Not { operand } => Expression::Not {
            operand: Box::new(substitute_param(*operand, from_index, substitute)),
        },

        Expression::IsNull { operand } => Expression::IsNull {
            operand: Box::new(substitute_param(*operand, from_index, substitute)),
        },
        Expression::IsNotNull { operand } => Expression::IsNotNull {
            operand: Box::new(substitute_param(*operand, from_index, substitute)),
        },

        Expression::HostCall {
            function_name,
            args,
        } => Expression::HostCall {
            function_name,
            args: args
                .into_iter()
                .map(|arg| substitute_param(arg, from_index, substitute))
                .collect(),
        },

        Expression::ListContains { list, element } => Expression::ListContains {
            list: Box::new(substitute_param(*list, from_index, substitute)),
            element: Box::new(substitute_param(*element, from_index, substitute)),
        },

        Expression::IfThenElse {
            condition,
            then_branch,
            else_branch,
        } => Expression::IfThenElse {
            condition: Box::new(substitute_param(*condition, from_index, substitute)),
            then_branch: Box::new(substitute_param(*then_branch, from_index, substitute)),
            else_branch: Box::new(substitute_param(*else_branch, from_index, substitute)),
        },

        // Literals don't contain params, return as-is
        Expression::Null | Expression::BoolLiteral { .. } | Expression::IntLiteral { .. } => expr,
    }
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

    // Try to get the source code
    let source_result = inspect.call_method1("getsource", (callable,));

    match source_result {
        Ok(source) => {
            let source_str: String = source.extract()?;
            analyze_lambda_source(py, &source_str, callable, param_count, class_hint)
        }
        Err(_) => {
            // Can't get source - try bytecode analysis as fallback
            analyze_lambda_bytecode(py, callable, param_count, class_hint)
        }
    }
}

/// Analyze lambda from bytecode when source code is unavailable.
///
/// This uses Python's dis module to disassemble the lambda's code object
/// and reconstruct an Expression tree from the bytecode instructions.
fn analyze_lambda_bytecode(
    py: Python<'_>,
    callable: &Py<PyAny>,
    _param_count: usize,
    class_hint: &str,
) -> PyResult<Expression> {
    let callable_bound = callable.bind(py);

    // Use dis module to get instructions - argval contains the resolved values
    let dis = py.import("dis")?;
    let get_instructions = dis.getattr("get_instructions")?;
    let instructions_iter = get_instructions.call1((callable_bound,))?;
    let instructions_list: Vec<Bound<'_, PyAny>> = instructions_iter
        .try_iter()?
        .collect::<Result<Vec<_>, _>>()?;

    // Stack-based evaluation
    let mut stack: Vec<BytecodeValue> = Vec::new();
    let class_name = class_hint.to_string();

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
                let code = callable_bound.getattr("__code__")?;
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
                let code = callable_bound.getattr("__code__")?;
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
    callable: &Py<PyAny>,
    param_count: usize,
    class_hint: &str,
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
            return analyze_lambda_bytecode(py, callable, param_count, class_hint);
        }
    };

    // Walk the AST to find the lambda expression
    let body = tree.getattr("body")?;

    // Extract lambda node and convert to Expression
    match extract_lambda_expression(py, &body, param_count, class_hint)? {
        Some(expr) => Ok(expr),
        None => {
            // Fallback to bytecode analysis if AST extraction fails
            analyze_lambda_bytecode(py, callable, param_count, class_hint)
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
    class_hint: &str,
) -> PyResult<Option<Expression>> {
    let node_type = node.get_type().name()?.to_string();
    let class_name = class_hint.to_string();

    match node_type.as_str() {
        "Attribute" => {
            // Field access: x.field
            let value = node.getattr("value")?;
            let attr: String = node.getattr("attr")?.extract()?;

            if let Some(base_expr) = convert_ast_to_expression(py, &value, arg_names, class_hint)? {
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
            convert_unaryop_to_expression(py, node, arg_names, class_hint)
        }

        "BinOp" => {
            // Binary operation: +, -, *, /
            convert_binop_to_expression(py, node, arg_names, class_hint)
        }

        "Constant" | "Num" | "NameConstant" => {
            // Literal value
            convert_constant_to_expression(node)
        }

        "Call" => {
            // Method call: obj.method() or function()
            let func = node.getattr("func")?;
            let func_type = func.get_type().name()?.to_string();

            if func_type == "Attribute" {
                // Method call: obj.method()
                let value = func.getattr("value")?;
                let method_name: String = func.getattr("attr")?.extract()?;

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

                    // Try to inline the method - look it up in the registry and analyze
                    if let Some(method) = get_method_from_class(py, class_hint, &method_name) {
                        if let Ok(method_body) = analyze_method_body(py, &method, class_hint) {
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
                                    inlined = substitute_param(inlined, (i + 1) as u32, &arg_expr);
                                }
                            }

                            return Ok(Some(inlined));
                        }
                    }

                    // Fallback: Create HostCall with class_method naming convention
                    let function_name = format!("{}_{}", class_name, method_name);
                    Ok(Some(Expression::HostCall {
                        function_name,
                        args: call_args,
                    }))
                } else {
                    Ok(None)
                }
            } else {
                // Regular function call - not supported in this context
                Ok(None)
            }
        }

        _ => Ok(None),
    }
}

/// Convert Python Compare AST node to Expression.
fn convert_compare_to_expression(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    arg_names: &[String],
    class_hint: &str,
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

    let left_expr = convert_ast_to_expression(py, &left, arg_names, class_hint)?;
    let right_expr = convert_ast_to_expression(py, &comparators[0], arg_names, class_hint)?;

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
    class_hint: &str,
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
        if let Some(expr) = convert_ast_to_expression(py, val, arg_names, class_hint)? {
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
    class_hint: &str,
) -> PyResult<Option<Expression>> {
    let op = node.getattr("op")?;
    let operand = node.getattr("operand")?;

    let op_type = op.get_type().name()?.to_string();

    if let Some(operand_expr) = convert_ast_to_expression(py, &operand, arg_names, class_hint)? {
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
    class_hint: &str,
) -> PyResult<Option<Expression>> {
    let op = node.getattr("op")?;
    let left = node.getattr("left")?;
    let right = node.getattr("right")?;

    let left_expr = convert_ast_to_expression(py, &left, arg_names, class_hint)?;
    let right_expr = convert_ast_to_expression(py, &right, arg_names, class_hint)?;

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
        pyo3::Python::initialize();
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
            py.run(c"f = lambda x: x.field", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();
            assert_eq!(info.param_count, 1);
        });
    }

    #[test]
    fn test_lambda_info_param_count_two() {
        init_python();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(c"f = lambda a, b: a.field", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();
            assert_eq!(info.param_count, 2);
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

            let info = LambdaInfo::new(py, func.unbind(), "test", "Lesson").unwrap();

            match &info.expression {
                Expression::FieldAccess {
                    field_name,
                    class_name,
                    ..
                } => {
                    assert_eq!(field_name, "timeslot");
                    assert_eq!(class_name, "Lesson");
                }
                _ => panic!("Expected FieldAccess, got {:?}", info.expression),
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

            let info = LambdaInfo::new(py, func.unbind(), "test", "Lesson").unwrap();

            match &info.expression {
                Expression::IsNotNull { operand } => match operand.as_ref() {
                    Expression::FieldAccess { field_name, .. } => {
                        assert_eq!(field_name, "room");
                    }
                    _ => panic!("Expected FieldAccess inside IsNotNull"),
                },
                _ => panic!("Expected IsNotNull, got {:?}", info.expression),
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

            let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();

            assert!(matches!(info.expression, Expression::IsNull { .. }));
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

            let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();

            match &info.expression {
                Expression::Gt { left, right } => {
                    assert!(matches!(left.as_ref(), Expression::FieldAccess { .. }));
                    assert!(matches!(
                        right.as_ref(),
                        Expression::IntLiteral { value: 5 }
                    ));
                }
                _ => panic!("Expected Gt, got {:?}", info.expression),
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

            let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();

            assert!(matches!(info.expression, Expression::Eq { .. }));
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

            let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();

            assert!(matches!(info.expression, Expression::And { .. }));
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

            let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();

            assert!(matches!(info.expression, Expression::Or { .. }));
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

            let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();

            assert!(matches!(info.expression, Expression::Not { .. }));
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

            let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();

            assert!(matches!(info.expression, Expression::Add { .. }));
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

            let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();

            assert!(matches!(info.expression, Expression::Sub { .. }));
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

            let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();

            assert!(matches!(info.expression, Expression::Mul { .. }));
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

            let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();

            assert!(matches!(info.expression, Expression::Div { .. }));
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

            let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();

            match &info.expression {
                Expression::Eq { left, right } => {
                    // Verify both sides are field accesses from different params
                    match (left.as_ref(), right.as_ref()) {
                        (
                            Expression::FieldAccess {
                                object: left_obj, ..
                            },
                            Expression::FieldAccess {
                                object: right_obj, ..
                            },
                        ) => {
                            assert!(matches!(left_obj.as_ref(), Expression::Param { index: 0 }));
                            assert!(matches!(right_obj.as_ref(), Expression::Param { index: 1 }));
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

            let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();

            match &info.expression {
                Expression::Add { left, right } => {
                    assert!(matches!(left.as_ref(), Expression::Param { index: 0 }));
                    assert!(matches!(right.as_ref(), Expression::Param { index: 1 }));
                }
                _ => panic!("Expected Add expression, got {:?}", info.expression),
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

            let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();

            // Should be Add(Add(a, b), c)
            match &info.expression {
                Expression::Add { left, right } => {
                    // right should be Param 2 (c)
                    assert!(matches!(right.as_ref(), Expression::Param { index: 2 }));
                    // left should be Add(a, b)
                    match left.as_ref() {
                        Expression::Add {
                            left: l2,
                            right: r2,
                        } => {
                            assert!(matches!(l2.as_ref(), Expression::Param { index: 0 }));
                            assert!(matches!(r2.as_ref(), Expression::Param { index: 1 }));
                        }
                        _ => panic!("Expected nested Add"),
                    }
                }
                _ => panic!("Expected Add expression, got {:?}", info.expression),
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

            let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();

            match &info.expression {
                Expression::FieldAccess {
                    field_name, object, ..
                } => {
                    assert_eq!(field_name, "name");
                    // The object should be another FieldAccess
                    match object.as_ref() {
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

            let info = LambdaInfo::new(py, func.unbind(), "test", "Entity").unwrap();

            // Expression should be populated with FieldAccess
            assert!(matches!(info.expression, Expression::FieldAccess { .. }));
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

            let info = LambdaInfo::new(py, func.unbind(), "equal_map", "Entity").unwrap();
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

    // ========================================================================
    // Method Introspection Tests
    // ========================================================================

    #[test]
    fn test_register_and_get_method_from_class() {
        init_python();
        Python::attach(|py| {
            // Clear registry from previous tests
            clear_class_registry();

            // Define a simple class with a method
            let locals = PyDict::new(py);
            py.run(
                c"class Vehicle:\n    def get_capacity(self):\n        return self.capacity",
                None,
                Some(&locals),
            )
            .unwrap();
            let vehicle_class = locals.get_item("Vehicle").unwrap().unwrap();

            // Register the class
            register_class(py, "Vehicle", &vehicle_class);

            // Look up the method
            let method = get_method_from_class(py, "Vehicle", "get_capacity");
            assert!(method.is_some());
        });
    }

    #[test]
    fn test_get_method_from_unregistered_class() {
        init_python();
        Python::attach(|py| {
            clear_class_registry();

            // Should return None for unregistered class
            let method = get_method_from_class(py, "UnknownClass", "some_method");
            assert!(method.is_none());
        });
    }

    #[test]
    fn test_get_nonexistent_method() {
        init_python();
        Python::attach(|py| {
            clear_class_registry();

            // Define a class without the method we'll look for
            let locals = PyDict::new(py);
            py.run(c"class Vehicle:\n    capacity = 100", None, Some(&locals))
                .unwrap();
            let vehicle_class = locals.get_item("Vehicle").unwrap().unwrap();

            register_class(py, "Vehicle", &vehicle_class);

            // Should return None for non-existent method
            let method = get_method_from_class(py, "Vehicle", "nonexistent_method");
            assert!(method.is_none());
        });
    }

    #[test]
    fn test_analyze_method_body_simple_field_return() {
        init_python();
        Python::attach(|py| {
            clear_class_registry();

            // Define a method that returns a field
            let locals = PyDict::new(py);
            py.run(
                c"class Vehicle:\n    def get_capacity(self):\n        return self.capacity",
                None,
                Some(&locals),
            )
            .unwrap();
            let vehicle_class = locals.get_item("Vehicle").unwrap().unwrap();
            register_class(py, "Vehicle", &vehicle_class);

            let method = get_method_from_class(py, "Vehicle", "get_capacity").unwrap();
            let expr = analyze_method_body(py, &method, "Vehicle").unwrap();

            // Should be FieldAccess on self (param 0)
            match expr {
                Expression::FieldAccess {
                    object,
                    field_name,
                    class_name,
                } => {
                    assert_eq!(field_name, "capacity");
                    assert_eq!(class_name, "Vehicle");
                    assert!(matches!(*object, Expression::Param { index: 0 }));
                }
                _ => panic!("Expected FieldAccess, got {:?}", expr),
            }
        });
    }

    #[test]
    fn test_analyze_method_body_arithmetic() {
        init_python();
        Python::attach(|py| {
            clear_class_registry();

            // Define a method with arithmetic: self.demand - self.capacity
            let locals = PyDict::new(py);
            py.run(
                c"class Vehicle:\n    def get_excess(self):\n        return self.demand - self.capacity",
                None,
                Some(&locals),
            )
            .unwrap();
            let vehicle_class = locals.get_item("Vehicle").unwrap().unwrap();
            register_class(py, "Vehicle", &vehicle_class);

            let method = get_method_from_class(py, "Vehicle", "get_excess").unwrap();
            let expr = analyze_method_body(py, &method, "Vehicle").unwrap();

            // Should be Sub(FieldAccess(demand), FieldAccess(capacity))
            match expr {
                Expression::Sub { left, right } => {
                    match *left {
                        Expression::FieldAccess { field_name, .. } => {
                            assert_eq!(field_name, "demand");
                        }
                        _ => panic!("Expected FieldAccess on left"),
                    }
                    match *right {
                        Expression::FieldAccess { field_name, .. } => {
                            assert_eq!(field_name, "capacity");
                        }
                        _ => panic!("Expected FieldAccess on right"),
                    }
                }
                _ => panic!("Expected Sub expression, got {:?}", expr),
            }
        });
    }

    #[test]
    fn test_analyze_method_body_with_param() {
        init_python();
        Python::attach(|py| {
            clear_class_registry();

            // Define a method with extra parameter: def add_value(self, x): return self.value + x
            let locals = PyDict::new(py);
            py.run(
                c"class Entity:\n    def add_value(self, x):\n        return self.value + x",
                None,
                Some(&locals),
            )
            .unwrap();
            let entity_class = locals.get_item("Entity").unwrap().unwrap();
            register_class(py, "Entity", &entity_class);

            let method = get_method_from_class(py, "Entity", "add_value").unwrap();
            let expr = analyze_method_body(py, &method, "Entity").unwrap();

            // Should be Add(FieldAccess(self.value), Param(1))
            match expr {
                Expression::Add { left, right } => {
                    match *left {
                        Expression::FieldAccess { field_name, .. } => {
                            assert_eq!(field_name, "value");
                        }
                        _ => panic!("Expected FieldAccess on left"),
                    }
                    // x is param index 1 (self is 0)
                    assert!(matches!(*right, Expression::Param { index: 1 }));
                }
                _ => panic!("Expected Add expression, got {:?}", expr),
            }
        });
    }

    // ========================================================================
    // Expression Substitution Tests
    // ========================================================================

    #[test]
    fn test_substitute_param_simple() {
        // Param(0) -> FieldAccess
        let expr = Expression::Param { index: 0 };
        let substitute = Expression::FieldAccess {
            object: Box::new(Expression::Param { index: 0 }),
            class_name: "Vehicle".to_string(),
            field_name: "id".to_string(),
        };

        let result = substitute_param(expr, 0, &substitute);
        assert!(matches!(result, Expression::FieldAccess { .. }));
    }

    #[test]
    fn test_substitute_param_no_match() {
        // Param(1) should not be replaced when substituting index 0
        let expr = Expression::Param { index: 1 };
        let substitute = Expression::IntLiteral { value: 42 };

        let result = substitute_param(expr, 0, &substitute);
        assert!(matches!(result, Expression::Param { index: 1 }));
    }

    #[test]
    fn test_substitute_param_in_field_access() {
        // FieldAccess(Param(0), "capacity") -> FieldAccess(FieldAccess(Param(0), "vehicle"), "capacity")
        let expr = Expression::FieldAccess {
            object: Box::new(Expression::Param { index: 0 }),
            class_name: "Vehicle".to_string(),
            field_name: "capacity".to_string(),
        };
        let substitute = Expression::FieldAccess {
            object: Box::new(Expression::Param { index: 0 }),
            class_name: "Route".to_string(),
            field_name: "vehicle".to_string(),
        };

        let result = substitute_param(expr, 0, &substitute);
        match result {
            Expression::FieldAccess {
                object, field_name, ..
            } => {
                assert_eq!(field_name, "capacity");
                // The object should now be the substitute (another FieldAccess)
                assert!(matches!(*object, Expression::FieldAccess { .. }));
            }
            _ => panic!("Expected FieldAccess"),
        }
    }

    #[test]
    fn test_substitute_param_in_binary_op() {
        // Add(Param(0), IntLiteral(10)) with Param(0) -> FieldAccess
        let expr = Expression::Add {
            left: Box::new(Expression::Param { index: 0 }),
            right: Box::new(Expression::IntLiteral { value: 10 }),
        };
        let substitute = Expression::FieldAccess {
            object: Box::new(Expression::Param { index: 0 }),
            class_name: "Entity".to_string(),
            field_name: "value".to_string(),
        };

        let result = substitute_param(expr, 0, &substitute);
        match result {
            Expression::Add { left, right } => {
                assert!(matches!(*left, Expression::FieldAccess { .. }));
                assert!(matches!(*right, Expression::IntLiteral { value: 10 }));
            }
            _ => panic!("Expected Add"),
        }
    }

    #[test]
    fn test_substitute_param_preserves_literals() {
        let expr = Expression::IntLiteral { value: 42 };
        let substitute = Expression::Param { index: 99 };

        let result = substitute_param(expr, 0, &substitute);
        assert!(matches!(result, Expression::IntLiteral { value: 42 }));
    }

    #[test]
    fn test_substitute_param_method_inlining_scenario() {
        // Simulate method inlining:
        // Method: def get_excess(self): return self.demand - self.capacity
        // Analyzed as: Sub(FieldAccess(Param(0), "demand"), FieldAccess(Param(0), "capacity"))
        //
        // Lambda: lambda v: v.get_excess() > 0
        // When inlining, we substitute Param(0) in method body with Param(0) from lambda
        // (which represents 'v')

        let method_body = Expression::Sub {
            left: Box::new(Expression::FieldAccess {
                object: Box::new(Expression::Param { index: 0 }), // self
                class_name: "Vehicle".to_string(),
                field_name: "demand".to_string(),
            }),
            right: Box::new(Expression::FieldAccess {
                object: Box::new(Expression::Param { index: 0 }), // self
                class_name: "Vehicle".to_string(),
                field_name: "capacity".to_string(),
            }),
        };

        // The calling object in lambda is Param(0) (the 'v' parameter)
        let calling_object = Expression::Param { index: 0 };

        // After substitution, self references become lambda parameter references
        let inlined = substitute_param(method_body, 0, &calling_object);

        match inlined {
            Expression::Sub { left, right } => {
                // Both should still be FieldAccess with Param(0) as object
                match (*left, *right) {
                    (
                        Expression::FieldAccess {
                            object: l_obj,
                            field_name: l_field,
                            ..
                        },
                        Expression::FieldAccess {
                            object: r_obj,
                            field_name: r_field,
                            ..
                        },
                    ) => {
                        assert_eq!(l_field, "demand");
                        assert_eq!(r_field, "capacity");
                        assert!(matches!(*l_obj, Expression::Param { index: 0 }));
                        assert!(matches!(*r_obj, Expression::Param { index: 0 }));
                    }
                    _ => panic!("Expected FieldAccess on both sides"),
                }
            }
            _ => panic!("Expected Sub expression"),
        }
    }

    // ========================================================================
    // AST Method Inlining Tests
    // ========================================================================

    #[test]
    fn test_ast_method_call_no_inlining_when_unregistered() {
        init_python();
        Python::attach(|py| {
            clear_class_registry();

            // Lambda that calls an unregistered method: lambda v: v.get_name()
            let locals = PyDict::new(py);
            py.run(
                c"
lambda_func = lambda v: v.get_name()
",
                None,
                Some(&locals),
            )
            .unwrap();

            let lambda_obj = locals.get_item("lambda_func").unwrap().unwrap();
            let lambda_info =
                LambdaInfo::new(py, lambda_obj.clone().unbind(), "test", "Entity").unwrap();

            // Should create HostCall since get_name is not registered
            match &lambda_info.expression {
                Expression::HostCall {
                    function_name,
                    args,
                } => {
                    assert_eq!(function_name, "Entity_get_name");
                    assert_eq!(args.len(), 1); // Just the object
                }
                _ => panic!(
                    "Expected HostCall when method not registered, got {:?}",
                    lambda_info.expression
                ),
            }
        });
    }

    #[test]
    fn test_ast_method_call_with_inlining() {
        init_python();
        Python::attach(|py| {
            clear_class_registry();

            // Register Entity class with a method
            let locals = PyDict::new(py);
            py.run(
                c"
class Entity:
    def is_available(self):
        return self.status == 'active'

lambda_func = lambda e: e.is_available()
",
                None,
                Some(&locals),
            )
            .unwrap();

            let entity_class = locals.get_item("Entity").unwrap().unwrap();
            register_class(py, "Entity", &entity_class);

            let lambda_obj = locals.get_item("lambda_func").unwrap().unwrap();
            let lambda_info =
                LambdaInfo::new(py, lambda_obj.clone().unbind(), "filter", "Entity").unwrap();

            // Should inline the method and produce Eq comparison, not HostCall
            match &lambda_info.expression {
                Expression::Eq { left, right } => {
                    // Left should be FieldAccess to status
                    match **left {
                        Expression::FieldAccess { ref field_name, .. } => {
                            assert_eq!(field_name, "status");
                        }
                        _ => panic!("Expected FieldAccess on left side"),
                    }
                    // Right should be String literal "active"
                    assert!(matches!(**right, Expression::StringLiteral { .. }));
                }
                Expression::HostCall { .. } => {
                    panic!("Method should have been inlined, got HostCall");
                }
                _ => panic!("Expected Eq or HostCall, got {:?}", lambda_info.expression),
            }
        });
    }

    #[test]
    fn test_ast_method_call_with_arguments() {
        init_python();
        Python::attach(|py| {
            clear_class_registry();

            // Register Entity class with a method that takes arguments
            let locals = PyDict::new(py);
            py.run(
                c"
class Entity:
    def check_value(self, threshold):
        return self.value > threshold

lambda_func = lambda e, t: e.check_value(t)
",
                None,
                Some(&locals),
            )
            .unwrap();

            let entity_class = locals.get_item("Entity").unwrap().unwrap();
            register_class(py, "Entity", &entity_class);

            let lambda_obj = locals.get_item("lambda_func").unwrap().unwrap();
            let lambda_info =
                LambdaInfo::new(py, lambda_obj.clone().unbind(), "filter", "Entity").unwrap();

            // Should inline the method with parameter substitution
            match &lambda_info.expression {
                Expression::Gt { left, right } => {
                    // Left should be FieldAccess to value
                    match **left {
                        Expression::FieldAccess { ref field_name, .. } => {
                            assert_eq!(field_name, "value");
                        }
                        _ => panic!("Expected FieldAccess on left"),
                    }
                    // Right should be Param(1) (the threshold argument)
                    assert!(matches!(**right, Expression::Param { index: 1 }));
                }
                _ => panic!("Expected Gt comparison, got {:?}", lambda_info.expression),
            }
        });
    }

    #[test]
    fn test_ast_method_call_inlined_in_comparison() {
        init_python();
        Python::attach(|py| {
            clear_class_registry();

            // Register class and create lambda with method call in comparison
            let locals = PyDict::new(py);
            py.run(
                c"
class Entity:
    def get_priority(self):
        return self.priority

lambda_func = lambda e: e.get_priority() > 5
",
                None,
                Some(&locals),
            )
            .unwrap();

            let entity_class = locals.get_item("Entity").unwrap().unwrap();
            register_class(py, "Entity", &entity_class);

            let lambda_obj = locals.get_item("lambda_func").unwrap().unwrap();
            let lambda_info =
                LambdaInfo::new(py, lambda_obj.clone().unbind(), "filter", "Entity").unwrap();

            // Should produce Gt(FieldAccess(priority), IntLiteral(5))
            match &lambda_info.expression {
                Expression::Gt { left, right } => {
                    match **left {
                        Expression::FieldAccess { ref field_name, .. } => {
                            assert_eq!(field_name, "priority");
                        }
                        _ => panic!("Expected FieldAccess"),
                    }
                    assert!(matches!(**right, Expression::IntLiteral { value: 5 }));
                }
                _ => panic!("Expected Gt expression"),
            }
        });
    }
}
