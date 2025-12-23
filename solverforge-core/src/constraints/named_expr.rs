//! Named expressions for constraint stream integration.
//!
//! Provides `NamedExpression` which bundles an `Expression` tree with an auto-generated
//! function name for use in constraint streams. This bridges the expression builder API
//! with the constraint stream API.
//!
//! # Example
//!
//! ```ignore
//! use solverforge_core::wasm::{Expr, FieldAccessExt};
//! use solverforge_core::constraints::{NamedExpression, StreamComponent};
//!
//! // Build an expression with auto-generated name
//! let has_room = NamedExpression::new(
//!     Expr::is_not_null(Expr::param(0).get("Lesson", "room"))
//! );
//!
//! // Use directly in stream components
//! let filter = StreamComponent::filter(has_room.into());
//! ```

use std::sync::atomic::{AtomicU64, Ordering};

use crate::constraints::WasmFunction;
use crate::wasm::Expression;

/// Counter for generating unique function names
static EXPR_COUNTER: AtomicU64 = AtomicU64::new(0);

/// An expression bundled with a function name for use in constraint streams.
///
/// `NamedExpression` automatically generates unique function names for expressions,
/// making it easy to use expressions in constraint stream components.
///
/// The generated function name follows the pattern `expr_{counter}` or uses a
/// provided custom name.
#[derive(Debug, Clone, PartialEq)]
pub struct NamedExpression {
    name: String,
    expression: Expression,
}

impl NamedExpression {
    /// Creates a new named expression with an auto-generated unique name.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let expr = NamedExpression::new(Expr::is_not_null(Expr::param(0)));
    /// // Name will be something like "expr_0", "expr_1", etc.
    /// ```
    pub fn new(expression: Expression) -> Self {
        let counter = EXPR_COUNTER.fetch_add(1, Ordering::SeqCst);
        Self {
            name: format!("expr_{}", counter),
            expression,
        }
    }

    /// Creates a named expression with a specific name.
    ///
    /// Use this when you want to give your expression a meaningful name
    /// for debugging or readability.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let has_room = NamedExpression::with_name(
    ///     "lesson_has_room",
    ///     Expr::is_not_null(Expr::param(0).get("Lesson", "room"))
    /// );
    /// ```
    pub fn with_name(name: impl Into<String>, expression: Expression) -> Self {
        Self {
            name: name.into(),
            expression,
        }
    }

    /// Returns the function name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the expression.
    pub fn expression(&self) -> &Expression {
        &self.expression
    }

    /// Consumes self and returns the expression.
    pub fn into_expression(self) -> Expression {
        self.expression
    }

    /// Returns a tuple of (name, expression) for registration with WasmModuleBuilder.
    pub fn into_parts(self) -> (String, Expression) {
        (self.name, self.expression)
    }
}

impl From<NamedExpression> for WasmFunction {
    fn from(named: NamedExpression) -> WasmFunction {
        WasmFunction::new(named.name)
    }
}

impl From<&NamedExpression> for WasmFunction {
    fn from(named: &NamedExpression) -> WasmFunction {
        WasmFunction::new(&named.name)
    }
}

/// Extension trait for converting expressions to named expressions.
pub trait IntoNamedExpression {
    /// Converts to a NamedExpression with an auto-generated name.
    fn named(self) -> NamedExpression;

    /// Converts to a NamedExpression with the given name.
    fn named_as(self, name: impl Into<String>) -> NamedExpression;
}

impl IntoNamedExpression for Expression {
    fn named(self) -> NamedExpression {
        NamedExpression::new(self)
    }

    fn named_as(self, name: impl Into<String>) -> NamedExpression {
        NamedExpression::with_name(name, self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wasm::{Expr, FieldAccessExt};

    #[test]
    fn test_named_expression_new() {
        let expr = Expr::is_not_null(Expr::param(0));
        let named = NamedExpression::new(expr.clone());

        assert!(named.name().starts_with("expr_"));
        assert_eq!(named.expression(), &expr);
    }

    #[test]
    fn test_named_expression_with_name() {
        let expr = Expr::is_not_null(Expr::param(0));
        let named = NamedExpression::with_name("my_predicate", expr.clone());

        assert_eq!(named.name(), "my_predicate");
        assert_eq!(named.expression(), &expr);
    }

    #[test]
    fn test_unique_names() {
        let expr1 = NamedExpression::new(Expr::bool(true));
        let expr2 = NamedExpression::new(Expr::bool(false));

        assert_ne!(expr1.name(), expr2.name());
    }

    #[test]
    fn test_into_wasm_function() {
        let named = NamedExpression::with_name("test_fn", Expr::bool(true));
        let wasm_fn: WasmFunction = named.into();

        assert_eq!(wasm_fn.name(), "test_fn");
    }

    #[test]
    fn test_into_parts() {
        let expr = Expr::int(42);
        let named = NamedExpression::with_name("answer", expr.clone());
        let (name, expression) = named.into_parts();

        assert_eq!(name, "answer");
        assert_eq!(expression, expr);
    }

    #[test]
    fn test_extension_trait() {
        let expr = Expr::is_not_null(Expr::param(0).get("Lesson", "room"));
        let named = expr.clone().named();

        assert!(named.name().starts_with("expr_"));
        assert_eq!(named.expression(), &expr);
    }

    #[test]
    fn test_extension_trait_named_as() {
        let expr = Expr::is_not_null(Expr::param(0).get("Lesson", "room"));
        let named = expr.clone().named_as("has_room");

        assert_eq!(named.name(), "has_room");
        assert_eq!(named.expression(), &expr);
    }

    #[test]
    fn test_complex_expression() {
        // Build: lesson.room != null && lesson.timeslot != null
        let has_room = Expr::is_not_null(Expr::param(0).get("Lesson", "room"));
        let has_timeslot = Expr::is_not_null(Expr::param(0).get("Lesson", "timeslot"));
        let both_assigned = Expr::and(has_room, has_timeslot);

        let named = both_assigned.named_as("lesson_fully_assigned");

        assert_eq!(named.name(), "lesson_fully_assigned");
        match named.expression() {
            Expression::And { .. } => {}
            _ => panic!("Expected And expression"),
        }
    }
}
