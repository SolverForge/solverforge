use crate::wasm::expression::Expression;
use wasm_encoder::ValType;

/// Definition of a predicate function to be compiled into WASM.
#[derive(Debug, Clone)]
pub struct PredicateDefinition {
    pub name: String,
    pub arity: u32,
    pub body: Expression,
    /// Parameter types for the predicate function. If None, defaults to all i32.
    pub param_types: Option<Vec<ValType>>,
}

impl PredicateDefinition {
    pub fn from_expression(name: impl Into<String>, arity: u32, expression: Expression) -> Self {
        Self {
            name: name.into(),
            arity,
            body: expression,
            param_types: None,
        }
    }

    /// Create a predicate from an expression with explicit parameter types.
    /// Useful for functions that take non-i32 parameters (e.g., f32 for loadBalance unfairness).
    pub fn from_expression_with_types(
        name: impl Into<String>,
        param_types: Vec<ValType>,
        expression: Expression,
    ) -> Self {
        Self {
            name: name.into(),
            arity: param_types.len() as u32,
            body: expression,
            param_types: Some(param_types),
        }
    }
}
