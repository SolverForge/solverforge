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
            // Source unavailable, use bytecode analysis instead
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
            let stmts: Vec<Bound<'_, PyAny>> = body_list.iter().collect();

            for (i, stmt) in stmts.iter().enumerate() {
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
                // Handle If statements with returns
                if stmt_type == "If" {
                    // First try standard if/else extraction
                    if let Ok(expr) = try_extract_if_expression(py, stmt, &arg_names, class_hint) {
                        return Ok(expr);
                    }
                    // Try if-early-return pattern: if condition returns, rest is else
                    if let Ok(expr) = try_extract_if_early_return(
                        py,
                        stmt,
                        &stmts[i + 1..],
                        &arg_names,
                        class_hint,
                    ) {
                        return Ok(expr);
                    }
                }
            }

            // Try to recognize common patterns with loops (e.g., sum accumulation)
            if let Ok(expr) =
                try_extract_accumulation_pattern(py, body_list, &arg_names, class_hint)
            {
                return Ok(expr);
            }

            // Try sequential expression substitution pattern:
            // var1 = expr1; var2 = expr2(var1); return method(var2)
            if let Ok(expr) =
                try_extract_sequential_expression_pattern(py, &stmts, &arg_names, class_hint)
            {
                return Ok(expr);
            }

            // Try assignment-based pattern for shadow variable update methods:
            // if cond1: self.field = val1
            // elif cond2: self.field = val2
            // else: self.field = val3
            if let Ok(expr) = try_extract_assignment_pattern(py, &stmts, &arg_names, class_hint) {
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

        // Math functions
        Expression::Sqrt { operand } => Expression::Sqrt {
            operand: Box::new(substitute_param(*operand, from_index, substitute)),
        },
        Expression::FloatAbs { operand } => Expression::FloatAbs {
            operand: Box::new(substitute_param(*operand, from_index, substitute)),
        },
        Expression::Round { operand } => Expression::Round {
            operand: Box::new(substitute_param(*operand, from_index, substitute)),
        },
        Expression::Floor { operand } => Expression::Floor {
            operand: Box::new(substitute_param(*operand, from_index, substitute)),
        },
        Expression::Ceil { operand } => Expression::Ceil {
            operand: Box::new(substitute_param(*operand, from_index, substitute)),
        },
        Expression::Sin { operand } => Expression::Sin {
            operand: Box::new(substitute_param(*operand, from_index, substitute)),
        },
        Expression::Cos { operand } => Expression::Cos {
            operand: Box::new(substitute_param(*operand, from_index, substitute)),
        },
        Expression::Asin { operand } => Expression::Asin {
            operand: Box::new(substitute_param(*operand, from_index, substitute)),
        },
        Expression::Acos { operand } => Expression::Acos {
            operand: Box::new(substitute_param(*operand, from_index, substitute)),
        },
        Expression::Atan { operand } => Expression::Atan {
            operand: Box::new(substitute_param(*operand, from_index, substitute)),
        },
        Expression::Atan2 { y, x } => Expression::Atan2 {
            y: Box::new(substitute_param(*y, from_index, substitute)),
            x: Box::new(substitute_param(*x, from_index, substitute)),
        },
        Expression::Radians { operand } => Expression::Radians {
            operand: Box::new(substitute_param(*operand, from_index, substitute)),
        },
        Expression::IntToFloat { operand } => Expression::IntToFloat {
            operand: Box::new(substitute_param(*operand, from_index, substitute)),
        },
        Expression::FloatToInt { operand } => Expression::FloatToInt {
            operand: Box::new(substitute_param(*operand, from_index, substitute)),
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

        Expression::Length { collection } => Expression::Length {
            collection: Box::new(substitute_param(*collection, from_index, substitute)),
        },

        Expression::Sum {
            collection,
            item_var_name,
            item_param_index,
            item_class_name,
            accumulator_expr,
        } => {
            // When substituting, we need to adjust the item_param_index if necessary
            // If from_index <= item_param_index, we shifted indices, so decrement
            let new_index = if from_index < item_param_index {
                item_param_index - 1
            } else {
                item_param_index
            };

            Expression::Sum {
                collection: Box::new(substitute_param(*collection, from_index, substitute)),
                item_var_name,
                item_param_index: new_index,
                item_class_name,
                accumulator_expr: Box::new(substitute_param(
                    *accumulator_expr,
                    from_index,
                    substitute,
                )),
            }
        }

        Expression::LastElement {
            collection,
            item_class_name,
        } => Expression::LastElement {
            collection: Box::new(substitute_param(*collection, from_index, substitute)),
            item_class_name,
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
        Expression::Null
        | Expression::BoolLiteral { .. }
        | Expression::IntLiteral { .. }
        | Expression::FloatLiteral { .. }
        | Expression::StringLiteral { .. } => expr,
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

/// Attempt to inline a method call in bytecode analysis.
///
/// Given an object (stack value) and method name, tries to:
/// 1. Look up the method from the class registry
/// 2. Analyze the method body
/// 3. Substitute parameters and return inlined expression
///
/// If inlining fails but we can convert the object and args to expressions,
/// returns a MethodCall expression that will be resolved via pre-computed lookup.
///
/// Returns Some(Expression) on success (inlined or MethodCall), None on failure.
fn try_inline_method_from_bytecode(
    py: Python<'_>,
    object: &BytecodeValue,
    method_name: &str,
    args: &[BytecodeValue],
    class_name: &str,
) -> Option<Expression> {
    // Try to look up the method and inline it
    if let Some(method) = get_method_from_class(py, class_name, method_name) {
        // Try to analyze the method body
        if let Ok(method_body) = analyze_method_body(py, &method, class_name) {
            // Convert object to expression
            if let Ok(object_expr) = bytecode_value_to_expression(object.clone()) {
                let mut inlined = method_body;

                // Substitute self (Param(0))
                inlined = substitute_param(inlined, 0, &object_expr);

                // Substitute other parameters
                for (i, arg) in args.iter().enumerate() {
                    if let Ok(arg_expr) = bytecode_value_to_expression(arg.clone()) {
                        inlined = substitute_param(inlined, (i + 1) as u32, &arg_expr);
                    }
                }

                return Some(inlined);
            }
        }
    }

    // Method not found or inlining failed - return None
    // The caller will return an appropriate error message
    None
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
            "LOAD_ATTR" => {
                // Field/method access - argval is the attribute name directly
                // In Python 3.11+, LOAD_ATTR handles both fields and methods:
                // - arg & 1 == 0: field access
                // - arg & 1 == 1: method load (pushes NULL for CALL)
                let attr_name: String = argval.extract()?;
                let arg: i32 = instr.getattr("arg")?.extract().unwrap_or(0);
                let is_method_load = (arg & 1) == 1;

                if let Some(obj) = stack.pop() {
                    if is_method_load {
                        // Method reference - mark for potential inlining at CALL time
                        stack.push(BytecodeValue::MethodRef {
                            object: Box::new(obj),
                            method_name: attr_name,
                        });
                    } else {
                        // Regular field access
                        stack.push(BytecodeValue::FieldAccess {
                            object: Box::new(obj),
                            class_name: class_name.clone(),
                            field_name: attr_name,
                        });
                    }
                }
            }
            "LOAD_METHOD" => {
                // Method reference - mark it for potential inlining at CALL time
                let method_name: String = argval.extract()?;
                if let Some(obj) = stack.pop() {
                    stack.push(BytecodeValue::MethodRef {
                        object: Box::new(obj),
                        method_name,
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
                } else if let Ok(s) = argval.extract::<String>() {
                    stack.push(BytecodeValue::String(s));
                } else if let Ok(f) = argval.extract::<f64>() {
                    stack.push(BytecodeValue::Float(f));
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
            "CALL_METHOD" => {
                // Python 3.10 and earlier: Call a method that was loaded with LOAD_METHOD
                // argval is the number of arguments
                let arg_count: i32 = argval.extract()?;
                if stack.len() >= (arg_count + 1) as usize {
                    let mut args = Vec::new();
                    for _ in 0..arg_count {
                        if let Some(arg) = stack.pop() {
                            args.insert(0, arg);
                        }
                    }

                    if let Some(method_ref) = stack.pop() {
                        if let BytecodeValue::MethodRef {
                            object,
                            method_name,
                        } = method_ref
                        {
                            // Try to inline the method
                            if let Some(inlined) = try_inline_method_from_bytecode(
                                py,
                                &object,
                                &method_name,
                                &args,
                                &class_name,
                            ) {
                                stack.push(BytecodeValue::InlinedExpression(Box::new(inlined)));
                            } else {
                                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                                    format!(
                                        "Cannot inline method {}.{}()",
                                        class_name, method_name
                                    ),
                                ));
                            }
                        } else {
                            stack.push(method_ref);
                        }
                    }
                }
            }
            "CALL" => {
                // Python 3.11+: Call function - argval is argument count
                // When calling a method ref, it will be preceded by the method and object on stack
                let arg_count: i32 = argval.extract()?;
                if stack.len() >= (arg_count + 1) as usize {
                    let mut args = Vec::new();
                    for _ in 0..arg_count {
                        if let Some(arg) = stack.pop() {
                            args.insert(0, arg);
                        }
                    }

                    if let Some(func_or_ref) = stack.pop() {
                        if let BytecodeValue::MethodRef {
                            object,
                            method_name,
                        } = func_or_ref
                        {
                            // Try to inline the method
                            if let Some(inlined) = try_inline_method_from_bytecode(
                                py,
                                &object,
                                &method_name,
                                &args,
                                &class_name,
                            ) {
                                stack.push(BytecodeValue::InlinedExpression(Box::new(inlined)));
                            } else {
                                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                                    format!(
                                        "Cannot inline method {}.{}()",
                                        class_name, method_name
                                    ),
                                ));
                            }
                        } else {
                            stack.push(func_or_ref);
                        }
                    }
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
    String(String),
    Float(f64),
    FieldAccess {
        object: Box<BytecodeValue>,
        class_name: String,
        field_name: String,
    },
    // Temporary marker for method reference - used during method call analysis
    MethodRef {
        object: Box<BytecodeValue>,
        method_name: String,
    },
    // Stores an inlined expression from method inlining
    InlinedExpression(Box<Expression>),
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
        BytecodeValue::String(s) => Ok(Expression::StringLiteral { value: s }),
        BytecodeValue::Float(f) => Ok(Expression::FloatLiteral { value: f }),
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
        BytecodeValue::MethodRef { .. } => {
            // Method reference that wasn't inlined - shouldn't reach here
            Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Cannot analyze lambda: unresolved method reference.",
            ))
        }
        BytecodeValue::InlinedExpression(expr) => {
            // Successfully inlined expression
            Ok(*expr)
        }
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

/// Extract if-then-else expression from an If statement.
///
/// Handles patterns like:
/// ```python
/// if condition:
///     return expr1
/// else:
///     return expr2
/// ```
fn try_extract_if_expression(
    py: Python<'_>,
    if_stmt: &Bound<'_, PyAny>,
    arg_names: &[String],
    class_hint: &str,
) -> PyResult<Expression> {
    // Extract condition
    let condition_node = if_stmt.getattr("test")?;
    let condition = convert_ast_to_expression(py, &condition_node, arg_names, class_hint)?
        .ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("Cannot convert if condition")
        })?;

    // Extract then branch (body)
    let body = if_stmt.getattr("body")?;
    let body_list = body.cast::<PyList>()?;
    let mut then_expr = None;
    for stmt in body_list.iter() {
        let stmt_type = stmt.get_type().name()?.to_string();
        if stmt_type == "Return" {
            let value = stmt.getattr("value")?;
            if !value.is_none() {
                then_expr = convert_ast_to_expression(py, &value, arg_names, class_hint)?;
                break;
            } else {
                then_expr = Some(Expression::Null);
                break;
            }
        }
    }

    let then_expr = then_expr.ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>("If statement body must contain return")
    })?;

    // Extract else branch (orelse)
    let orelse = if_stmt.getattr("orelse")?;
    let orelse_list = orelse.cast::<PyList>()?;
    let mut else_expr = None;

    for stmt in orelse_list.iter() {
        let stmt_type = stmt.get_type().name()?.to_string();
        if stmt_type == "Return" {
            let value = stmt.getattr("value")?;
            if !value.is_none() {
                else_expr = convert_ast_to_expression(py, &value, arg_names, class_hint)?;
                break;
            } else {
                else_expr = Some(Expression::Null);
                break;
            }
        } else if stmt_type == "If" {
            // Nested if - recursively extract
            else_expr = Some(try_extract_if_expression(py, &stmt, arg_names, class_hint)?);
            break;
        }
    }

    let else_expr = else_expr.ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "If statement must have else branch with return",
        )
    })?;

    Ok(Expression::IfThenElse {
        condition: Box::new(condition),
        then_branch: Box::new(then_expr),
        else_branch: Box::new(else_expr),
    })
}

/// Extract assignment-based pattern for methods that assign to self.field instead of returning.
///
/// Handles patterns like:
/// ```python
/// if condition1:
///     self.field = expr1
/// elif condition2:
///     self.field = expr2
/// else:
///     self.field = expr3
/// ```
///
/// The assigned expressions are extracted and combined into a conditional expression.
fn try_extract_assignment_pattern(
    py: Python<'_>,
    stmts: &[Bound<'_, PyAny>],
    arg_names: &[String],
    class_hint: &str,
) -> PyResult<Expression> {
    // Look for an If statement that contains assignments
    for stmt in stmts {
        let stmt_type = stmt.get_type().name()?.to_string();
        if stmt_type == "If" {
            if let Ok(expr) = try_extract_if_assignment(py, stmt, arg_names, class_hint) {
                return Ok(expr);
            }
        }
    }
    Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
        "No assignment pattern found",
    ))
}

/// Extract expression from if/elif/else that assigns to self.field.
fn try_extract_if_assignment(
    py: Python<'_>,
    if_stmt: &Bound<'_, PyAny>,
    arg_names: &[String],
    class_hint: &str,
) -> PyResult<Expression> {
    // Extract condition
    let condition_node = if_stmt.getattr("test")?;
    let condition = convert_ast_to_expression(py, &condition_node, arg_names, class_hint)?
        .ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("Cannot convert if condition")
        })?;

    // Extract then branch - look for assignment or return
    let body = if_stmt.getattr("body")?;
    let body_list = body.cast::<PyList>()?;
    let then_expr = extract_branch_value(py, body_list, arg_names, class_hint)?;

    // Extract else branch
    let orelse = if_stmt.getattr("orelse")?;
    let orelse_list = orelse.cast::<PyList>()?;

    let else_expr = if orelse_list.is_empty() {
        // No else branch - this shouldn't happen for complete assignment patterns
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Assignment pattern requires else branch",
        ));
    } else {
        // Check if it's an elif (nested If) or else block
        let first_stmt = orelse_list.get_item(0)?;
        let first_type = first_stmt.get_type().name()?.to_string();
        if first_type == "If" {
            // Recursively handle elif
            try_extract_if_assignment(py, &first_stmt, arg_names, class_hint)?
        } else {
            // Extract from else block
            extract_branch_value(py, orelse_list, arg_names, class_hint)?
        }
    };

    Ok(Expression::IfThenElse {
        condition: Box::new(condition),
        then_branch: Box::new(then_expr),
        else_branch: Box::new(else_expr),
    })
}

/// Extract the value expression from a branch (handles both Return and Assign to self.field).
fn extract_branch_value(
    py: Python<'_>,
    stmts: &Bound<'_, PyList>,
    arg_names: &[String],
    class_hint: &str,
) -> PyResult<Expression> {
    for stmt in stmts.iter() {
        let stmt_type = stmt.get_type().name()?.to_string();

        if stmt_type == "Return" {
            let value = stmt.getattr("value")?;
            if value.is_none() {
                return Ok(Expression::Null);
            }
            return convert_ast_to_expression(py, &value, arg_names, class_hint)?.ok_or_else(
                || PyErr::new::<pyo3::exceptions::PyValueError, _>("Cannot convert return value"),
            );
        }

        if stmt_type == "Assign" {
            // Check if it's assigning to self.field
            let targets = stmt.getattr("targets")?;
            let targets_list = targets.cast::<PyList>()?;
            if !targets_list.is_empty() {
                let target = targets_list.get_item(0)?;
                let target_type = target.get_type().name()?.to_string();
                if target_type == "Attribute" {
                    let target_value = target.getattr("value")?;
                    let target_value_type = target_value.get_type().name()?.to_string();
                    if target_value_type == "Name" {
                        let name: String = target_value.getattr("id")?.extract()?;
                        if name == "self" || arg_names.first() == Some(&name) {
                            // This is self.field = expr, extract the value
                            let value = stmt.getattr("value")?;
                            if value.is_none() {
                                return Ok(Expression::Null);
                            }
                            return convert_ast_to_expression(py, &value, arg_names, class_hint)?
                                .ok_or_else(|| {
                                    PyErr::new::<pyo3::exceptions::PyValueError, _>(
                                        "Cannot convert assigned value",
                                    )
                                });
                        }
                    }
                }
            }
        }

        if stmt_type == "Expr" {
            // Expression statement - might be a None assignment represented differently
            let value = stmt.getattr("value")?;
            let value_type = value.get_type().name()?.to_string();
            if value_type == "Constant" || value_type == "NameConstant" {
                if let Ok(is_none) = value.is_none().then_some(true).ok_or(false) {
                    if is_none {
                        return Ok(Expression::Null);
                    }
                }
            }
        }
    }

    Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
        "Branch must contain return or assignment to self.field",
    ))
}

/// Extract if-early-return pattern where if body returns and remaining statements are the else.
///
/// Handles patterns like:
/// ```python
/// if condition:
///     return x
/// return y  # This is the implicit else
/// ```
fn try_extract_if_early_return(
    py: Python<'_>,
    if_stmt: &Bound<'_, PyAny>,
    remaining_stmts: &[Bound<'_, PyAny>],
    arg_names: &[String],
    class_hint: &str,
) -> PyResult<Expression> {
    // Extract condition
    let condition_node = if_stmt.getattr("test")?;
    let condition_opt = convert_ast_to_expression(py, &condition_node, arg_names, class_hint)?;

    // Extract then branch from if body
    let body = if_stmt.getattr("body")?;
    let body_list = body.cast::<PyList>()?;
    let mut then_expr = None;
    for stmt in body_list.iter() {
        let stmt_type = stmt.get_type().name()?.to_string();
        if stmt_type == "Return" {
            let value = stmt.getattr("value")?;
            if !value.is_none() {
                then_expr = convert_ast_to_expression(py, &value, arg_names, class_hint)?;
                break;
            } else {
                then_expr = Some(Expression::Null);
                break;
            }
        }
    }

    let then_expr = then_expr.ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "If body must contain return for early-return pattern",
        )
    })?;

    // Check for patterns where condition can't be converted but we can use a fallback:
    // 1. "empty collection guard": if len(collection) == 0: return 0 - use Sum
    // 2. "optional check": if X is not None: return A; return B - use fallback B
    log::debug!(
        "try_extract_if_early_return: condition_opt={:?}, then_expr={:?}",
        condition_opt,
        then_expr
    );

    // Pattern 1: Empty collection guard - works even if condition can be converted
    let is_guard = is_empty_collection_guard(py, &condition_node)?;
    let is_zero = matches!(then_expr, Expression::IntLiteral { value: 0 });
    log::debug!(
        "Empty guard check: is_guard={}, is_zero={}, then_expr={:?}",
        is_guard,
        is_zero,
        then_expr
    );

    if is_guard && is_zero {
        // Try accumulation pattern - Sum handles empty collections naturally
        let remaining_list = PyList::new(py, remaining_stmts)?;
        log::debug!(
            "Trying accumulation pattern on {} remaining statements",
            remaining_stmts.len()
        );
        match try_extract_accumulation_pattern(py, &remaining_list, arg_names, class_hint) {
            Ok(accum_expr) => {
                log::debug!("Accumulation pattern succeeded: {:?}", accum_expr);
                return Ok(accum_expr);
            }
            Err(e) => {
                log::debug!("Accumulation pattern failed: {}", e);
            }
        }
    }

    if condition_opt.is_none() {
        // Pattern 2: "if X is not None: ...; return fallback" - use the fallback
        // This handles cases like ClassVar access that we can't convert
        if is_not_none_check(py, &condition_node)? {
            // The remaining statements should have a fallback return
            for stmt in remaining_stmts.iter() {
                let stmt_type = stmt.get_type().name()?.to_string();
                if stmt_type == "Return" {
                    let value = stmt.getattr("value")?;
                    if !value.is_none() {
                        if let Some(fallback_expr) =
                            convert_ast_to_expression(py, &value, arg_names, class_hint)?
                        {
                            log::debug!("Using fallback for 'is not None' pattern");
                            return Ok(fallback_expr);
                        }
                    }
                }
            }
        }

        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Cannot convert if condition",
        ));
    }

    let condition = condition_opt.unwrap();

    // Extract else from remaining statements - find a return or accumulation pattern
    let mut else_expr = None;
    for stmt in remaining_stmts.iter() {
        let stmt_type = stmt.get_type().name()?.to_string();
        if stmt_type == "Return" {
            let value = stmt.getattr("value")?;
            if !value.is_none() {
                else_expr = convert_ast_to_expression(py, &value, arg_names, class_hint)?;
                break;
            }
        } else if stmt_type == "If" {
            // Nested if-early-return in else branch
            let remaining_after =
                &remaining_stmts[remaining_stmts.iter().position(|s| s.is(stmt)).unwrap() + 1..];
            else_expr = Some(try_extract_if_early_return(
                py,
                stmt,
                remaining_after,
                arg_names,
                class_hint,
            )?);
            break;
        }
    }

    // If no direct return found, try accumulation pattern on remaining statements
    if else_expr.is_none() {
        let remaining_list = PyList::new(py, remaining_stmts)?;
        if let Ok(accum_expr) =
            try_extract_accumulation_pattern(py, &remaining_list, arg_names, class_hint)
        {
            else_expr = Some(accum_expr);
        }
    }

    let else_expr = else_expr.ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Remaining statements must contain return or accumulation pattern",
        )
    })?;

    Ok(Expression::IfThenElse {
        condition: Box::new(condition),
        then_branch: Box::new(then_expr),
        else_branch: Box::new(else_expr),
    })
}

/// Check if a condition AST node is an "empty collection guard" pattern: len(x) == 0
fn is_empty_collection_guard(py: Python<'_>, condition: &Bound<'_, PyAny>) -> PyResult<bool> {
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
    if comp_type != "Constant" && comp_type != "Num" {
        return Ok(false);
    }

    if let Ok(value) = comp.getattr("value") {
        if let Ok(0i64) = value.extract() {
            return Ok(true);
        }
    }

    let _ = py;
    Ok(false)
}

/// Check if a condition AST node is an "is not None" check: X is not None
fn is_not_none_check(py: Python<'_>, condition: &Bound<'_, PyAny>) -> PyResult<bool> {
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
    if comp_type == "Constant" || comp_type == "NameConstant" {
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

    let _ = py;
    Ok(false)
}

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
fn try_extract_accumulation_pattern(
    py: Python<'_>,
    body_list: &Bound<'_, PyList>,
    arg_names: &[String],
    class_hint: &str,
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
                    if let Some((cond, early_val)) =
                        try_extract_early_return_if(py, &stmt, arg_names, class_hint)?
                    {
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
                            if value_type == "Constant" || value_type == "Num" {
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

/// Try to extract expression from sequential variable assignments.
///
/// Matches patterns like:
/// ```python
/// var1 = expr1
/// var2 = func(var1)
/// return method(var2)
/// ```
///
/// This is converted by substituting local variables with their expressions.
fn try_extract_sequential_expression_pattern(
    py: Python<'_>,
    stmts: &[Bound<'_, PyAny>],
    arg_names: &[String],
    class_hint: &str,
) -> PyResult<Expression> {
    // Collect variable assignments: var_name -> AST expression
    let mut local_vars: std::collections::HashMap<String, Bound<'_, PyAny>> =
        std::collections::HashMap::new();
    let mut return_node: Option<Bound<'_, PyAny>> = None;
    let mut early_return_if: Option<(Expression, Expression)> = None;

    for (idx, stmt) in stmts.iter().enumerate() {
        let stmt_type = stmt.get_type().name()?.to_string();

        match stmt_type.as_str() {
            "If" => {
                // Handle early return pattern at the start
                if idx == 0 {
                    if let Some((cond, ret_val)) =
                        try_extract_early_return_if(py, stmt, arg_names, class_hint)?
                    {
                        early_return_if = Some((cond, ret_val));
                    }
                }
            }
            "Assign" => {
                let targets = stmt.getattr("targets")?;
                let targets_list = targets.cast::<PyList>()?;
                if targets_list.len() == 1 {
                    let target = targets_list.get_item(0)?;
                    let target_type = target.get_type().name()?.to_string();
                    if target_type == "Name" {
                        let var_name: String = target.getattr("id")?.extract()?;
                        let value = stmt.getattr("value")?;
                        local_vars.insert(var_name, value);
                    }
                }
            }
            "Return" => {
                let value = stmt.getattr("value")?;
                if !value.is_none() {
                    return_node = Some(value);
                }
            }
            _ => {}
        }
    }

    // Must have a return statement
    let return_ast = return_node.ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>("No return statement found")
    })?;

    log::debug!(
        "try_extract_sequential_expression_pattern: {} local vars, return_ast type: {}",
        local_vars.len(),
        return_ast
            .get_type()
            .name()
            .map(|n| n.to_string())
            .unwrap_or_default()
    );

    // Convert return expression with local variable substitution
    let result = convert_ast_with_local_var_substitution(
        py,
        &return_ast,
        arg_names,
        class_hint,
        &local_vars,
    )?;

    // Wrap with early return if present
    if let Some((condition, early_value)) = early_return_if {
        return Ok(Expression::IfThenElse {
            condition: Box::new(condition),
            then_branch: Box::new(early_value),
            else_branch: Box::new(result),
        });
    }

    Ok(result)
}

/// Convert AST to expression, substituting local variable references.
fn convert_ast_with_local_var_substitution(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    arg_names: &[String],
    class_hint: &str,
    local_vars: &std::collections::HashMap<String, Bound<'_, PyAny>>,
) -> PyResult<Expression> {
    let node_type = node.get_type().name()?.to_string();

    // Check if this is a local variable reference
    if node_type == "Name" {
        let var_name: String = node.getattr("id")?.extract()?;

        // If it's a local variable, substitute with its expression
        if let Some(var_expr_ast) = local_vars.get(&var_name) {
            return convert_ast_with_local_var_substitution(
                py,
                var_expr_ast,
                arg_names,
                class_hint,
                local_vars,
            );
        }
    }

    // Handle method calls specially - need to convert args with substitution
    if node_type == "Call" {
        let func = node.getattr("func")?;
        let func_type = func.get_type().name()?.to_string();

        if func_type == "Attribute" {
            let value = func.getattr("value")?;
            let method_name: String = func.getattr("attr")?.extract()?;

            // Convert the base object with substitution
            let base_expr = convert_ast_with_local_var_substitution(
                py, &value, arg_names, class_hint, local_vars,
            )?;

            // Convert arguments with substitution
            let call_args = node.getattr("args")?;
            let args_list = call_args.cast::<PyList>()?;
            let mut converted_args = Vec::new();
            for arg in args_list.iter() {
                let arg_expr = convert_ast_with_local_var_substitution(
                    py, &arg, arg_names, class_hint, local_vars,
                )?;
                converted_args.push(arg_expr);
            }

            // Infer base class and try to inline the method
            let base_class = infer_expression_class(py, &base_expr, class_hint)?
                .unwrap_or_else(|| class_hint.to_string());

            return build_method_call_expr(
                py,
                base_expr,
                &method_name,
                &converted_args,
                &base_class,
            );
        }

        // Handle function calls like math.radians, math.sin, etc.
        if func_type == "Attribute" || func_type == "Name" {
            // Fall through to standard conversion which handles math functions
        }
    }

    // Handle binary operations with substitution
    if node_type == "BinOp" {
        let left = node.getattr("left")?;
        let right = node.getattr("right")?;
        let op = node.getattr("op")?;
        let op_type = op.get_type().name()?.to_string();

        let left_expr =
            convert_ast_with_local_var_substitution(py, &left, arg_names, class_hint, local_vars)?;
        let right_expr =
            convert_ast_with_local_var_substitution(py, &right, arg_names, class_hint, local_vars)?;

        return Ok(match op_type.as_str() {
            "Add" => Expression::Add {
                left: Box::new(left_expr),
                right: Box::new(right_expr),
            },
            "Sub" => Expression::Sub {
                left: Box::new(left_expr),
                right: Box::new(right_expr),
            },
            "Mult" => Expression::Mul {
                left: Box::new(left_expr),
                right: Box::new(right_expr),
            },
            "Div" | "TrueDiv" => Expression::Div {
                left: Box::new(left_expr),
                right: Box::new(right_expr),
            },
            _ => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Unsupported binary operator: {}",
                    op_type
                )));
            }
        });
    }

    // Handle subscript (tuple indexing) with substitution
    if node_type == "Subscript" {
        let value = node.getattr("value")?;
        let slice = node.getattr("slice")?;

        // Check if value is a local variable (tuple)
        let value_type = value.get_type().name()?.to_string();
        if value_type == "Name" {
            let var_name: String = value.getattr("id")?.extract()?;
            if let Some(var_expr_ast) = local_vars.get(&var_name) {
                // This is indexing into a local variable (likely a tuple)
                // Check if the variable was assigned from a tuple expression
                let var_expr_type = var_expr_ast.get_type().name()?.to_string();

                if var_expr_type == "Tuple" {
                    // Get the index
                    let slice_type = slice.get_type().name()?.to_string();
                    if slice_type == "Constant" || slice_type == "Num" {
                        if let Ok(index) = slice.getattr("value").and_then(|v| v.extract::<i64>()) {
                            // Get the indexed element from the tuple
                            let elts = var_expr_ast.getattr("elts")?;
                            let elts_list = elts.cast::<PyList>()?;
                            if (index as usize) < elts_list.len() {
                                let element = elts_list.get_item(index as usize)?;
                                return convert_ast_with_local_var_substitution(
                                    py, &element, arg_names, class_hint, local_vars,
                                );
                            }
                        }
                    }
                }

                // If tuple var was assigned from a method call, we need to track which element
                // This is complex - for now, fall back to error
                if var_expr_type == "Call" {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                        "Tuple indexing from method call result not yet supported",
                    ));
                }
            }
        }
    }

    // Fall back to standard conversion
    convert_ast_to_expression(py, node, arg_names, class_hint)?
        .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("Cannot convert expression"))
}

/// Try to extract an early return from an If statement.
///
/// Matches pattern: `if condition: return value`
/// Returns (condition_expr, early_return_value) on success.
fn try_extract_early_return_if(
    py: Python<'_>,
    if_stmt: &Bound<'_, PyAny>,
    arg_names: &[String],
    class_hint: &str,
) -> PyResult<Option<(Expression, Expression)>> {
    let test = if_stmt.getattr("test")?;
    let body = if_stmt.getattr("body")?;
    let body_list = body.cast::<PyList>()?;
    let orelse = if_stmt.getattr("orelse")?;
    let orelse_list = orelse.cast::<PyList>()?;

    // Must have exactly one statement in body and empty else
    if body_list.len() != 1 || !orelse_list.is_empty() {
        return Ok(None);
    }

    let body_stmt = body_list.get_item(0)?;
    let stmt_type = body_stmt.get_type().name()?.to_string();

    // Body must be a Return statement
    if stmt_type != "Return" {
        return Ok(None);
    }

    let ret_value = body_stmt.getattr("value")?;
    if ret_value.is_none() {
        return Ok(None);
    }

    // Convert condition and return value to expressions
    let condition_expr = convert_ast_to_expression(py, &test, arg_names, class_hint)?;
    let return_expr = convert_ast_to_expression(py, &ret_value, arg_names, class_hint)?;

    if let (Some(cond), Some(ret)) = (condition_expr, return_expr) {
        Ok(Some((cond, ret)))
    } else {
        Ok(None)
    }
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
    convert_ast_to_expression_with_mutable_var_substitution(
        py,
        &value,
        arg_names,
        class_hint,
        collection_expr,
        item_class_name,
        mutable_vars,
    )
}

/// Convert AST to expression, substituting mutable loop variables with their final values.
///
/// For post-loop expressions, mutable vars like `previous_location` should refer to
/// the last element's field value: `LastElement(collection).field`
fn convert_ast_to_expression_with_mutable_var_substitution(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    arg_names: &[String],
    class_hint: &str,
    collection_expr: &Expression,
    item_class_name: &str,
    mutable_vars: &[MutableLoopVar],
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
                            if let Some(arg_expr) =
                                convert_ast_to_expression(py, &arg, arg_names, class_hint)?
                            {
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
                        return build_method_call_expr(
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
    convert_ast_to_expression(py, node, arg_names, class_hint)
}

/// Substitute occurrences of a named variable with an expression.
///
/// Note: Expression nodes don't currently track variable names, so this
/// returns the expression unchanged. The infrastructure is in place for
/// future enhancement when variable name tracking is added.
#[allow(dead_code)]
fn substitute_named_var(
    expr: &Expression,
    _var_name: &str,
    _replacement: &Expression,
) -> Expression {
    expr.clone()
}

/// Information about a mutable variable tracked across loop iterations.
/// Used to detect "previous element" patterns like:
/// ```python
/// prev = self.init_field
/// for item in collection:
///     use(prev)
///     prev = item.field
/// ```
#[derive(Debug, Clone)]
struct MutableLoopVar {
    /// The variable name (e.g., "previous_location")
    name: String,
    /// The initialization expression (e.g., self.home_location)
    init_expr: Bound<'static, PyAny>,
    /// The field being tracked from each item (e.g., "location" from visit.location)
    item_field: String,
}

/// Context from loop extraction, needed for post-loop term processing.
struct LoopContext {
    /// The Sum expression for the loop
    sum_expr: Expression,
    /// The collection being iterated
    collection_expr: Expression,
    /// The item class name
    item_class_name: String,
    /// Mutable loop variables (for post-loop term substitution)
    mutable_vars: Vec<MutableLoopVar>,
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
fn extract_sum_from_for_loop(
    py: Python<'_>,
    for_loop: &Bound<'_, PyAny>,
    accum_var: &str,
    arg_names: &[String],
    class_hint: &str,
    pre_loop_assigns: &[(String, Bound<'_, PyAny>)],
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
    if let Some(collection_expr) = convert_ast_to_expression(py, &iter_expr, arg_names, class_hint)?
    {
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
                            let accumulated_expr = convert_ast_to_expression_with_mutable_vars(
                                py,
                                &value,
                                &loop_arg_names,
                                &item_class_hint,
                                &loop_var,
                                &mutable_vars,
                                arg_names,
                                class_hint,
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
fn convert_ast_to_expression_with_mutable_vars(
    py: Python<'_>,
    node: &Bound<'_, PyAny>,
    arg_names: &[String],
    class_hint: &str,
    loop_var: &str,
    mutable_vars: &[MutableLoopVar],
    outer_arg_names: &[String],
    outer_class_hint: &str,
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
                            if let Some(arg_expr) =
                                convert_ast_to_expression(py, &arg, arg_names, class_hint)?
                            {
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
                            let init_base = convert_ast_to_expression(
                                py,
                                &mv.init_expr,
                                outer_arg_names,
                                outer_class_hint,
                            )?
                            .ok_or_else(|| {
                                PyErr::new::<pyo3::exceptions::PyValueError, _>(
                                    "Cannot convert init expr",
                                )
                            })?;

                            let then_branch = build_method_call_expr(
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

                            let else_branch = build_method_call_expr(
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
    convert_ast_to_expression(py, node, arg_names, class_hint)
}

/// Find the "previous" shadow variable for a class.
/// Returns the field name like "previous_visit" for Visit class.
fn find_previous_shadow_variable(py: Python<'_>, class_name: &str) -> PyResult<Option<String>> {
    let registry = CLASS_REGISTRY.read().unwrap();
    if let Some(ref map) = *registry {
        if let Some(class) = map.get(class_name) {
            let class_bound = class.bind(py);

            // Look for fields starting with "previous_"
            if let Ok(annotations) = class_bound.getattr("__annotations__") {
                if let Ok(keys) = annotations.call_method0("keys") {
                    for key in keys.try_iter()? {
                        let key_str: String = key?.extract()?;
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

/// Infer the class type of an expression.
/// For FieldAccess, looks up the field type from the parent class.
/// Returns the class name if determinable.
fn infer_expression_class(
    py: Python<'_>,
    expr: &Expression,
    default_class: &str,
) -> PyResult<Option<String>> {
    match expr {
        Expression::FieldAccess {
            class_name,
            field_name,
            ..
        } => {
            // Look up the field type from the class
            get_field_type_and_register(py, class_name, field_name)
        }
        Expression::Param { index } if *index == 0 => {
            // Parameter 0 is typically self, use the default class
            Ok(Some(default_class.to_string()))
        }
        _ => Ok(None),
    }
}

/// Recursively extract the concrete class name and type from a possibly nested generic type.
///
/// Handles patterns like:
/// - `SomeClass` -> ("SomeClass", SomeClass)
/// - `Optional[SomeClass]` -> ("SomeClass", SomeClass)
/// - `ClassVar[Optional[SomeClass]]` -> ("SomeClass", SomeClass)
/// - `list[SomeClass]` -> ("SomeClass", SomeClass)
fn extract_concrete_class_from_type<'py>(
    field_type: &Bound<'py, PyAny>,
) -> Option<(String, Bound<'py, PyAny>)> {
    // Check if it's a simple class (has __name__ that's a string, not a generic)
    if let Ok(type_name) = field_type.getattr("__name__") {
        if let Ok(name) = type_name.extract::<String>() {
            // Skip NoneType
            if name != "NoneType" {
                log::debug!("Found concrete class: {}", name);
                return Some((name, field_type.clone()));
            }
        }
    }

    // Check if it's a generic type with __args__ (like Optional[X], ClassVar[X], list[X])
    if let Ok(args) = field_type.getattr("__args__") {
        if let Ok(args_tuple) = args.cast::<pyo3::types::PyTuple>() {
            for arg in args_tuple.iter() {
                // Skip NoneType args (from Optional)
                if let Ok(arg_name) = arg.getattr("__name__") {
                    if let Ok(name) = arg_name.extract::<String>() {
                        if name == "NoneType" {
                            continue;
                        }
                    }
                }
                // Recursively try to extract from this arg
                if let Some(result) = extract_concrete_class_from_type(&arg) {
                    return Some(result);
                }
            }
        }
    }

    None
}

/// Look up a field's type from a class and register it if found.
/// Returns the class name of the field type.
fn get_field_type_and_register(
    py: Python<'_>,
    class_name: &str,
    field_name: &str,
) -> PyResult<Option<String>> {
    log::debug!(
        "get_field_type_and_register: looking up {}.{}",
        class_name,
        field_name
    );

    // First look up in registry
    let field_info: Option<(String, Py<PyAny>)> = {
        let registry = CLASS_REGISTRY.read().unwrap();
        if let Some(ref map) = *registry {
            if let Some(class) = map.get(class_name) {
                let class_bound = class.bind(py);

                // Get type hints from the class
                if let Ok(get_type_hints) = py
                    .import("typing")
                    .and_then(|m| m.getattr("get_type_hints"))
                {
                    if let Ok(hints) = get_type_hints.call1((&class_bound,)) {
                        if let Ok(field_type) = hints.get_item(field_name) {
                            log::debug!(
                                "Found field type for {}.{}: {:?}",
                                class_name,
                                field_name,
                                field_type
                            );
                            // Use recursive extraction to handle nested generics
                            if let Some((name, inner_type)) =
                                extract_concrete_class_from_type(&field_type)
                            {
                                log::debug!("Extracted concrete type: {}", name);
                                Some((name, inner_type.unbind()))
                            } else {
                                None
                            }
                        } else {
                            log::debug!(
                                "Field {} not found in hints for {}",
                                field_name,
                                class_name
                            );
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                log::debug!("Class {} not in registry", class_name);
                None
            }
        } else {
            None
        }
    };

    // Register the discovered class outside the read lock
    if let Some((ref field_class_name, ref field_class)) = field_info {
        let field_bound = field_class.bind(py);
        register_class(py, field_class_name, field_bound);
        log::debug!("Registered field class: {}", field_class_name);
        return Ok(Some(field_class_name.clone()));
    }

    Ok(None)
}

/// Infer the item type from a collection expression using Python type hints.
///
/// For a FieldAccess like Param(0).visits on class Vehicle, this inspects
/// the type hints of the Vehicle class to determine the item type (e.g., Visit).
fn infer_item_type(py: Python<'_>, collection_expr: &Expression) -> PyResult<String> {
    match collection_expr {
        Expression::FieldAccess {
            object: _,
            class_name,
            field_name,
        } => {
            // Get the class from the registry and extract item type info
            let item_info: Option<(String, Py<PyAny>)> = {
                let registry = CLASS_REGISTRY.read().unwrap();
                if let Some(ref map) = *registry {
                    if let Some(class) = map.get(class_name) {
                        let class_bound = class.bind(py);

                        // Get type hints from the class
                        if let Ok(get_type_hints) = py
                            .import("typing")
                            .and_then(|m| m.getattr("get_type_hints"))
                        {
                            if let Ok(hints) = get_type_hints.call1((&class_bound,)) {
                                if let Ok(field_type) = hints.get_item(field_name) {
                                    // field_type is something like typing.List[Visit]
                                    // Extract the inner type
                                    if let Ok(args) = field_type.getattr("__args__") {
                                        if let Ok(args_len) = args.len() {
                                            if args_len > 0 {
                                                if let Ok(item_type) = args.get_item(0) {
                                                    if let Ok(item_name) =
                                                        item_type.getattr("__name__")
                                                    {
                                                        if let Ok(item_class_name) =
                                                            item_name.extract::<String>()
                                                        {
                                                            Some((
                                                                item_class_name,
                                                                item_type.clone().unbind(),
                                                            ))
                                                        } else {
                                                            None
                                                        }
                                                    } else {
                                                        None
                                                    }
                                                } else {
                                                    None
                                                }
                                            } else {
                                                None
                                            }
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            // Register the discovered class (outside the read lock)
            if let Some((ref item_class_name, ref item_class)) = item_info {
                let item_bound = item_class.bind(py);
                register_class(py, item_class_name, item_bound);
                return Ok(item_class_name.clone());
            }

            // If type hints don't work, try field annotations on the instance
            let fallback_info: Option<(String, Option<Py<PyAny>>)> = {
                let registry = CLASS_REGISTRY.read().unwrap();
                if let Some(ref map) = *registry {
                    if let Some(class) = map.get(class_name) {
                        let class_bound = class.bind(py);

                        // Try __annotations__ directly
                        if let Ok(annotations) = class_bound.getattr("__annotations__") {
                            if let Ok(field_type) = annotations.get_item(field_name) {
                                // Try to get __name__ from the type (simple class reference)
                                if let Ok(item_name) = field_type.getattr("__name__") {
                                    if let Ok(item_class_name) = item_name.extract::<String>() {
                                        Some((item_class_name, Some(field_type.clone().unbind())))
                                    } else {
                                        None
                                    }
                                // Try to extract from typing generic
                                } else if let Ok(_origin) = field_type.getattr("__origin__") {
                                    if let Ok(args) = field_type.getattr("__args__") {
                                        if let Ok(args_len) = args.len() {
                                            if args_len > 0 {
                                                if let Ok(item_type) = args.get_item(0) {
                                                    if let Ok(item_name) =
                                                        item_type.getattr("__name__")
                                                    {
                                                        if let Ok(item_class_name) =
                                                            item_name.extract::<String>()
                                                        {
                                                            Some((
                                                                item_class_name,
                                                                Some(item_type.clone().unbind()),
                                                            ))
                                                        } else {
                                                            None
                                                        }
                                                    } else {
                                                        None
                                                    }
                                                } else {
                                                    None
                                                }
                                            } else {
                                                None
                                            }
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            if let Some((ref item_class_name, ref maybe_class)) = fallback_info {
                if let Some(ref item_class) = maybe_class {
                    let item_bound = item_class.bind(py);
                    register_class(py, item_class_name, item_bound);
                }
                return Ok(item_class_name.clone());
            }

            Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Cannot infer item type for field '{}.{}' - ensure it has type hints",
                class_name, field_name
            )))
        }
        _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Cannot infer item type from complex collection expression",
        )),
    }
}

/// Create a sum expression over a collection.
///
/// Constructs a Sum expression with:
/// - collection: The collection being iterated
/// - item_var_name: Name of the loop variable
/// - item_param_index: The parameter index assigned to the loop variable
/// - accumulator_expr: Expression being accumulated (uses loop variable as Param with item_param_index)
fn create_sum_over_collection(
    accumulated: Expression,
    loop_var: &str,
    collection: Expression,
    loop_var_param_index: u32,
    item_class_name: &str,
) -> Expression {
    Expression::Sum {
        collection: Box::new(collection),
        item_var_name: loop_var.to_string(),
        item_param_index: loop_var_param_index,
        item_class_name: item_class_name.to_string(),
        accumulator_expr: Box::new(accumulated),
    }
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
            // NOTE: Don't clear registry - causes race condition with parallel tests

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
            // NOTE: Don't clear registry - causes race condition with parallel tests

            // Should return None for unregistered class
            let method = get_method_from_class(py, "UnknownClass", "some_method");
            assert!(method.is_none());
        });
    }

    #[test]
    fn test_get_nonexistent_method() {
        init_python();
        Python::attach(|py| {
            // NOTE: Don't clear registry - causes race condition with parallel tests

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
            // NOTE: Don't clear registry - causes race condition with parallel tests

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
            // NOTE: Don't clear registry - causes race condition with parallel tests

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
            // NOTE: Don't clear registry - causes race condition with parallel tests

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
    fn test_ast_method_call_error_when_unregistered() {
        init_python();
        Python::attach(|py| {
            // NOTE: Don't clear registry - causes race condition with parallel tests

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
            // Should return error since get_name method cannot be inlined
            let result = LambdaInfo::new(py, lambda_obj.clone().unbind(), "test", "Entity");
            assert!(result.is_err(), "Expected error when method not registered");
            let err_msg = result.unwrap_err().to_string();
            assert!(
                err_msg.contains("Cannot inline method"),
                "Error should mention inlining failure: {}",
                err_msg
            );
        });
    }

    #[test]
    fn test_ast_method_call_with_inlining() {
        init_python();
        Python::attach(|py| {
            // NOTE: Don't clear registry - causes race condition with parallel tests

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

            // String comparisons ARE now inlined with StringLiteral support
            match &lambda_info.expression {
                Expression::Eq { left, right } => {
                    // Left should be FieldAccess to status
                    match **left {
                        Expression::FieldAccess { ref field_name, .. } => {
                            assert_eq!(field_name, "status");
                        }
                        _ => panic!("Expected FieldAccess on left, got {:?}", left),
                    }
                    // Right should be StringLiteral("active")
                    match **right {
                        Expression::StringLiteral { ref value } => {
                            assert_eq!(value, "active");
                        }
                        _ => panic!("Expected StringLiteral on right, got {:?}", right),
                    }
                }
                _ => panic!(
                    "Expected Eq expression with StringLiteral, got {:?}",
                    lambda_info.expression
                ),
            }
        });
    }

    #[test]
    fn test_ast_method_call_with_arguments() {
        init_python();
        Python::attach(|py| {
            // NOTE: Don't clear registry - causes race condition with parallel tests

            // Register class with a method that takes arguments
            // Use unique class name to avoid race condition with other tests
            let locals = PyDict::new(py);
            py.run(
                c"
class EntityWithArgs:
    def check_value(self, threshold):
        return self.value > threshold

lambda_func = lambda e, t: e.check_value(t)
",
                None,
                Some(&locals),
            )
            .unwrap();

            let entity_class = locals.get_item("EntityWithArgs").unwrap().unwrap();
            register_class(py, "EntityWithArgs", &entity_class);

            let lambda_obj = locals.get_item("lambda_func").unwrap().unwrap();
            let lambda_info =
                LambdaInfo::new(py, lambda_obj.clone().unbind(), "filter", "EntityWithArgs")
                    .unwrap();

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
            // NOTE: Don't clear registry - causes race condition with parallel tests
            // Use unique class name to avoid collision
            let locals = PyDict::new(py);
            py.run(
                c"
class EntityPriority:
    def get_priority(self):
        return self.priority

lambda_func = lambda e: e.get_priority() > 5
",
                None,
                Some(&locals),
            )
            .unwrap();

            let entity_class = locals.get_item("EntityPriority").unwrap().unwrap();
            register_class(py, "EntityPriority", &entity_class);

            let lambda_obj = locals.get_item("lambda_func").unwrap().unwrap();
            let lambda_info =
                LambdaInfo::new(py, lambda_obj.clone().unbind(), "filter", "EntityPriority")
                    .unwrap();

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

    // ========================================================================
    // Integration Tests for Method Analysis
    // ========================================================================

    #[test]
    fn test_integration_method_inlining_with_registration() {
        // Complete flow: register class -> create lambda with method call -> verify inlining
        init_python();
        Python::attach(|py| {
            // NOTE: Don't clear registry - causes race condition with parallel tests

            // Define and register a domain class
            let locals = PyDict::new(py);
            py.run(
                c"
class Vehicle:
    def is_valid(self):
        return self.status == 'valid'

lambda_func = lambda v: v.is_valid()
",
                None,
                Some(&locals),
            )
            .unwrap();

            let vehicle_class = locals.get_item("Vehicle").unwrap().unwrap();
            register_class(py, "Vehicle", &vehicle_class);

            // Verify the class is registered by looking it up
            let method = get_method_from_class(py, "Vehicle", "is_valid");
            assert!(
                method.is_some(),
                "Method should be found after registration"
            );

            // Analyze lambda with the method call
            let lambda_obj = locals.get_item("lambda_func").unwrap().unwrap();
            let lambda_info =
                LambdaInfo::new(py, lambda_obj.clone().unbind(), "filter", "Vehicle").unwrap();

            // Should have inlined the method call to an Eq expression
            match &lambda_info.expression {
                Expression::Eq { left, right: _ } => match **left {
                    Expression::FieldAccess { ref field_name, .. } => {
                        assert_eq!(field_name, "status");
                    }
                    _ => panic!("Expected field access"),
                },
                _ => panic!("Expected inlined Eq expression"),
            }
        });
    }

    #[test]
    fn test_integration_method_with_multiple_fields() {
        // Test inlining method that references multiple fields
        init_python();
        Python::attach(|py| {
            // NOTE: Don't clear registry - causes race condition with parallel tests

            let locals = PyDict::new(py);
            py.run(
                c"
class Shift:
    def is_overbooked(self):
        return self.hours > self.max_hours

lambda_func = lambda s: s.is_overbooked()
",
                None,
                Some(&locals),
            )
            .unwrap();

            let shift_class = locals.get_item("Shift").unwrap().unwrap();
            register_class(py, "Shift", &shift_class);

            let lambda_obj = locals.get_item("lambda_func").unwrap().unwrap();
            let lambda_info =
                LambdaInfo::new(py, lambda_obj.clone().unbind(), "filter", "Shift").unwrap();

            // Should inline to Gt(FieldAccess(hours), FieldAccess(max_hours))
            match &lambda_info.expression {
                Expression::Gt { left, right } => match (&**left, &**right) {
                    (
                        Expression::FieldAccess {
                            field_name: left_field,
                            ..
                        },
                        Expression::FieldAccess {
                            field_name: right_field,
                            ..
                        },
                    ) => {
                        assert_eq!(left_field, "hours");
                        assert_eq!(right_field, "max_hours");
                    }
                    _ => panic!("Expected FieldAccess on both sides"),
                },
                _ => panic!("Expected Gt expression"),
            }
        });
    }

    #[test]
    fn test_integration_method_chain_through_parameters() {
        // Test method call with arguments that get properly substituted
        init_python();
        Python::attach(|py| {
            // NOTE: Don't clear registry - causes race condition with parallel tests

            let locals = PyDict::new(py);
            py.run(
                c"
class Employee:
    def meets_minimum_salary(self, min_salary):
        return self.salary >= min_salary

lambda_func = lambda e, threshold: e.meets_minimum_salary(threshold)
",
                None,
                Some(&locals),
            )
            .unwrap();

            let employee_class = locals.get_item("Employee").unwrap().unwrap();
            register_class(py, "Employee", &employee_class);

            let lambda_obj = locals.get_item("lambda_func").unwrap().unwrap();
            let lambda_info =
                LambdaInfo::new(py, lambda_obj.clone().unbind(), "filter", "Employee").unwrap();

            // Should inline to Ge(FieldAccess(salary), Param(1))
            match &lambda_info.expression {
                Expression::Ge { left, right } => {
                    match (&**left, &**right) {
                        (
                            Expression::FieldAccess { field_name, .. },
                            Expression::Param { index },
                        ) if field_name == "salary" && *index == 1 => {
                            // Correct!
                        }
                        _ => panic!("Expected Ge(FieldAccess(salary), Param(1))"),
                    }
                }
                _ => panic!("Expected Ge expression, got {:?}", lambda_info.expression),
            }
        });
    }

    #[test]
    fn test_integration_registry_persistence() {
        // Test that registered classes persist across multiple lambda analyses
        // NOTE: Don't clear registry here - it causes race conditions with parallel tests
        init_python();
        Python::attach(|py| {
            // Use unique class name to avoid collision with other parallel tests
            let locals = PyDict::new(py);
            py.run(
                c"
class TaskPersistence:
    def is_completed(self):
        return self.status == 'done'

    def is_urgent(self):
        return self.priority > 5

lambda_completed = lambda t: t.is_completed()
lambda_urgent = lambda t: t.is_urgent()
",
                None,
                Some(&locals),
            )
            .unwrap();

            let task_class = locals.get_item("TaskPersistence").unwrap().unwrap();
            register_class(py, "TaskPersistence", &task_class);

            // Analyze first lambda
            let lambda1 = locals.get_item("lambda_completed").unwrap().unwrap();
            let info1 =
                LambdaInfo::new(py, lambda1.clone().unbind(), "filter", "TaskPersistence").unwrap();
            assert!(matches!(info1.expression, Expression::Eq { .. }));

            // Analyze second lambda - should still have access to registered class
            let lambda2 = locals.get_item("lambda_urgent").unwrap().unwrap();
            let info2 =
                LambdaInfo::new(py, lambda2.clone().unbind(), "filter", "TaskPersistence").unwrap();
            assert!(matches!(info2.expression, Expression::Gt { .. }));
        });
    }

    #[test]
    fn test_integration_bytecode_method_inlining() {
        // Test that bytecode analysis also supports method inlining
        init_python();
        Python::attach(|py| {
            // NOTE: Don't clear registry - causes race condition with parallel tests

            let locals = PyDict::new(py);
            py.run(
                c"
class Item:
    def has_stock(self):
        return self.quantity > 0

lambda_func = lambda i: i.has_stock()
",
                None,
                Some(&locals),
            )
            .unwrap();

            let item_class = locals.get_item("Item").unwrap().unwrap();
            register_class(py, "Item", &item_class);

            let lambda_obj = locals.get_item("lambda_func").unwrap().unwrap();
            // The lambda will be analyzed via either AST or bytecode
            let lambda_info =
                LambdaInfo::new(py, lambda_obj.clone().unbind(), "filter", "Item").unwrap();

            // Should produce a Gt expression from inlining
            assert!(
                matches!(lambda_info.expression, Expression::Gt { .. }),
                "Expected Gt expression from method inlining, got {:?}",
                lambda_info.expression
            );
        });
    }

    #[test]
    fn test_accumulation_with_early_return() {
        init_python();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(
                c"
class Visit:
    def __init__(self):
        self.demand = 10

class Vehicle:
    def __init__(self):
        self.visits = []

    def calculate_total_demand(self):
        if len(self.visits) == 0:
            return 0
        total = 0
        for visit in self.visits:
            total += visit.demand
        return total

lambda_func = lambda v: v.calculate_total_demand()
",
                None,
                Some(&locals),
            )
            .unwrap();

            let visit_class = locals.get_item("Visit").unwrap().unwrap();
            let vehicle_class = locals.get_item("Vehicle").unwrap().unwrap();
            register_class(py, "Visit", &visit_class);
            register_class(py, "Vehicle", &vehicle_class);

            let lambda_obj = locals.get_item("lambda_func").unwrap().unwrap();
            let result = LambdaInfo::new(py, lambda_obj.clone().unbind(), "demand", "Vehicle");

            // Should successfully analyze the accumulation pattern
            match result {
                Ok(info) => {
                    // Should be IfThenElse with Sum in else branch
                    assert!(
                        matches!(
                            info.expression,
                            Expression::IfThenElse { .. } | Expression::Sum { .. }
                        ),
                        "Expected IfThenElse or Sum, got {:?}",
                        info.expression
                    );
                }
                Err(e) => panic!("Failed to analyze accumulation pattern: {}", e),
            }
        });
    }
}
