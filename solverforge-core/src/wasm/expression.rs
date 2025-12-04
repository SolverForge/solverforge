use serde::{Deserialize, Serialize};

/// Rich expression tree for constraint predicates
///
/// This enum represents a complete expression language for building constraint predicates.
/// Expressions are serializable (via serde) for use across FFI boundaries.
///
/// # Example
/// ```
/// # use solverforge_core::wasm::Expression;
/// // Build expression: param(0).employee != null
/// let expr = Expression::IsNotNull {
///     operand: Box::new(Expression::FieldAccess {
///         object: Box::new(Expression::Param { index: 0 }),
///         class_name: "Shift".into(),
///         field_name: "employee".into(),
///     })
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind")]
pub enum Expression {
    // ===== Literals =====
    /// Integer literal (i64)
    IntLiteral { value: i64 },

    /// Boolean literal
    BoolLiteral { value: bool },

    /// Null value
    Null,

    // ===== Parameter Access =====
    /// Access a function parameter by index
    /// Example: param(0) refers to the first parameter
    Param { index: u32 },

    // ===== Field Access =====
    /// Access a field on an object
    /// Example: param(0).get("Employee", "name")
    FieldAccess {
        object: Box<Expression>,
        class_name: String,
        field_name: String,
    },

    // ===== Comparisons =====
    /// Equal comparison (==)
    Eq {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Not equal comparison (!=)
    Ne {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Less than comparison (<)
    Lt {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Less than or equal comparison (<=)
    Le {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Greater than comparison (>)
    Gt {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Greater than or equal comparison (>=)
    Ge {
        left: Box<Expression>,
        right: Box<Expression>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_int_literal() {
        let expr = Expression::IntLiteral { value: 42 };
        assert_eq!(expr, Expression::IntLiteral { value: 42 });
    }

    #[test]
    fn test_bool_literal() {
        let expr = Expression::BoolLiteral { value: true };
        assert_eq!(expr, Expression::BoolLiteral { value: true });
    }

    #[test]
    fn test_null() {
        let expr = Expression::Null;
        assert_eq!(expr, Expression::Null);
    }

    #[test]
    fn test_param() {
        let expr = Expression::Param { index: 0 };
        assert_eq!(expr, Expression::Param { index: 0 });
    }

    #[test]
    fn test_field_access() {
        let expr = Expression::FieldAccess {
            object: Box::new(Expression::Param { index: 0 }),
            class_name: "Employee".into(),
            field_name: "name".into(),
        };

        match expr {
            Expression::FieldAccess {
                object,
                class_name,
                field_name,
            } => {
                assert_eq!(class_name, "Employee");
                assert_eq!(field_name, "name");
                assert_eq!(*object, Expression::Param { index: 0 });
            }
            _ => panic!("Expected FieldAccess"),
        }
    }

    #[test]
    fn test_comparison_eq() {
        let expr = Expression::Eq {
            left: Box::new(Expression::IntLiteral { value: 1 }),
            right: Box::new(Expression::IntLiteral { value: 2 }),
        };

        match expr {
            Expression::Eq { left, right } => {
                assert_eq!(*left, Expression::IntLiteral { value: 1 });
                assert_eq!(*right, Expression::IntLiteral { value: 2 });
            }
            _ => panic!("Expected Eq"),
        }
    }

    #[test]
    fn test_serialize_int_literal() {
        let expr = Expression::IntLiteral { value: 42 };
        let json = serde_json::to_string(&expr).unwrap();
        assert!(json.contains("\"kind\":\"IntLiteral\""));
        assert!(json.contains("\"value\":42"));
    }

    #[test]
    fn test_deserialize_int_literal() {
        let json = r#"{"kind":"IntLiteral","value":42}"#;
        let expr: Expression = serde_json::from_str(json).unwrap();
        assert_eq!(expr, Expression::IntLiteral { value: 42 });
    }

    #[test]
    fn test_serialize_field_access() {
        let expr = Expression::FieldAccess {
            object: Box::new(Expression::Param { index: 0 }),
            class_name: "Employee".into(),
            field_name: "name".into(),
        };

        let json = serde_json::to_string(&expr).unwrap();
        let deserialized: Expression = serde_json::from_str(&json).unwrap();
        assert_eq!(expr, deserialized);
    }

    #[test]
    fn test_complex_expression() {
        // Build: param(0).employee != null
        let expr = Expression::Ne {
            left: Box::new(Expression::FieldAccess {
                object: Box::new(Expression::Param { index: 0 }),
                class_name: "Shift".into(),
                field_name: "employee".into(),
            }),
            right: Box::new(Expression::Null),
        };

        // Serialize and deserialize
        let json = serde_json::to_string(&expr).unwrap();
        let deserialized: Expression = serde_json::from_str(&json).unwrap();
        assert_eq!(expr, deserialized);
    }
}
