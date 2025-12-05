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

    // ===== Logical Operations =====
    /// Logical AND (&&)
    And {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Logical OR (||)
    Or {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Logical NOT (!)
    Not { operand: Box<Expression> },

    /// Null check (is null)
    IsNull { operand: Box<Expression> },

    /// Not-null check (is not null)
    IsNotNull { operand: Box<Expression> },

    // ===== Arithmetic Operations =====
    /// Addition (+)
    Add {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Subtraction (-)
    Sub {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Multiplication (*)
    Mul {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Division (/)
    Div {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    // ===== List Operations =====
    /// Check if a list contains an element
    /// Example: list.contains(element)
    ListContains {
        list: Box<Expression>,
        element: Box<Expression>,
    },

    // ===== Host Function Calls =====
    /// Call a host-provided function
    /// Example: hstringEquals(left, right)
    HostCall {
        function_name: String,
        args: Vec<Expression>,
    },

    // ===== Conditional =====
    /// If-then-else conditional expression
    /// Example: if condition { then_branch } else { else_branch }
    IfThenElse {
        condition: Box<Expression>,
        then_branch: Box<Expression>,
        else_branch: Box<Expression>,
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

    // ===== Logical Operations Tests =====

    #[test]
    fn test_logical_and() {
        let expr = Expression::And {
            left: Box::new(Expression::BoolLiteral { value: true }),
            right: Box::new(Expression::BoolLiteral { value: false }),
        };

        match expr {
            Expression::And { left, right } => {
                assert_eq!(*left, Expression::BoolLiteral { value: true });
                assert_eq!(*right, Expression::BoolLiteral { value: false });
            }
            _ => panic!("Expected And"),
        }
    }

    #[test]
    fn test_logical_or() {
        let expr = Expression::Or {
            left: Box::new(Expression::BoolLiteral { value: true }),
            right: Box::new(Expression::BoolLiteral { value: false }),
        };

        match expr {
            Expression::Or { left, right } => {
                assert_eq!(*left, Expression::BoolLiteral { value: true });
                assert_eq!(*right, Expression::BoolLiteral { value: false });
            }
            _ => panic!("Expected Or"),
        }
    }

    #[test]
    fn test_logical_not() {
        let expr = Expression::Not {
            operand: Box::new(Expression::BoolLiteral { value: true }),
        };

        match expr {
            Expression::Not { operand } => {
                assert_eq!(*operand, Expression::BoolLiteral { value: true });
            }
            _ => panic!("Expected Not"),
        }
    }

    #[test]
    fn test_is_null() {
        let expr = Expression::IsNull {
            operand: Box::new(Expression::Param { index: 0 }),
        };

        match expr {
            Expression::IsNull { operand } => {
                assert_eq!(*operand, Expression::Param { index: 0 });
            }
            _ => panic!("Expected IsNull"),
        }
    }

    #[test]
    fn test_is_not_null() {
        let expr = Expression::IsNotNull {
            operand: Box::new(Expression::Param { index: 0 }),
        };

        match expr {
            Expression::IsNotNull { operand } => {
                assert_eq!(*operand, Expression::Param { index: 0 });
            }
            _ => panic!("Expected IsNotNull"),
        }
    }

    #[test]
    fn test_serialize_logical_and() {
        let expr = Expression::And {
            left: Box::new(Expression::BoolLiteral { value: true }),
            right: Box::new(Expression::BoolLiteral { value: false }),
        };

        let json = serde_json::to_string(&expr).unwrap();
        let deserialized: Expression = serde_json::from_str(&json).unwrap();
        assert_eq!(expr, deserialized);
    }

    // ===== Arithmetic Operations Tests =====

    #[test]
    fn test_arithmetic_add() {
        let expr = Expression::Add {
            left: Box::new(Expression::IntLiteral { value: 10 }),
            right: Box::new(Expression::IntLiteral { value: 20 }),
        };

        match expr {
            Expression::Add { left, right } => {
                assert_eq!(*left, Expression::IntLiteral { value: 10 });
                assert_eq!(*right, Expression::IntLiteral { value: 20 });
            }
            _ => panic!("Expected Add"),
        }
    }

    #[test]
    fn test_arithmetic_sub() {
        let expr = Expression::Sub {
            left: Box::new(Expression::IntLiteral { value: 30 }),
            right: Box::new(Expression::IntLiteral { value: 10 }),
        };

        match expr {
            Expression::Sub { left, right } => {
                assert_eq!(*left, Expression::IntLiteral { value: 30 });
                assert_eq!(*right, Expression::IntLiteral { value: 10 });
            }
            _ => panic!("Expected Sub"),
        }
    }

    #[test]
    fn test_arithmetic_mul() {
        let expr = Expression::Mul {
            left: Box::new(Expression::IntLiteral { value: 5 }),
            right: Box::new(Expression::IntLiteral { value: 3 }),
        };

        match expr {
            Expression::Mul { left, right } => {
                assert_eq!(*left, Expression::IntLiteral { value: 5 });
                assert_eq!(*right, Expression::IntLiteral { value: 3 });
            }
            _ => panic!("Expected Mul"),
        }
    }

    #[test]
    fn test_arithmetic_div() {
        let expr = Expression::Div {
            left: Box::new(Expression::IntLiteral { value: 100 }),
            right: Box::new(Expression::IntLiteral { value: 5 }),
        };

        match expr {
            Expression::Div { left, right } => {
                assert_eq!(*left, Expression::IntLiteral { value: 100 });
                assert_eq!(*right, Expression::IntLiteral { value: 5 });
            }
            _ => panic!("Expected Div"),
        }
    }

    #[test]
    fn test_serialize_arithmetic() {
        let expr = Expression::Add {
            left: Box::new(Expression::IntLiteral { value: 10 }),
            right: Box::new(Expression::IntLiteral { value: 20 }),
        };

        let json = serde_json::to_string(&expr).unwrap();
        let deserialized: Expression = serde_json::from_str(&json).unwrap();
        assert_eq!(expr, deserialized);
    }

    #[test]
    fn test_complex_logical_expression() {
        // Build: (param(0).employee != null) && (param(0).skill == "Java")
        let expr = Expression::And {
            left: Box::new(Expression::IsNotNull {
                operand: Box::new(Expression::FieldAccess {
                    object: Box::new(Expression::Param { index: 0 }),
                    class_name: "Shift".into(),
                    field_name: "employee".into(),
                }),
            }),
            right: Box::new(Expression::Eq {
                left: Box::new(Expression::FieldAccess {
                    object: Box::new(Expression::Param { index: 0 }),
                    class_name: "Employee".into(),
                    field_name: "skill".into(),
                }),
                right: Box::new(Expression::IntLiteral { value: 42 }), // Placeholder
            }),
        };

        // Serialize and deserialize
        let json = serde_json::to_string(&expr).unwrap();
        let deserialized: Expression = serde_json::from_str(&json).unwrap();
        assert_eq!(expr, deserialized);
    }

    #[test]
    fn test_time_calculation_expression() {
        // Build: (shift.start / 24) to calculate day from hour
        let expr = Expression::Div {
            left: Box::new(Expression::FieldAccess {
                object: Box::new(Expression::Param { index: 0 }),
                class_name: "Shift".into(),
                field_name: "start".into(),
            }),
            right: Box::new(Expression::IntLiteral { value: 24 }),
        };

        // Serialize and deserialize
        let json = serde_json::to_string(&expr).unwrap();
        let deserialized: Expression = serde_json::from_str(&json).unwrap();
        assert_eq!(expr, deserialized);
    }

    // ===== Host Function Call Tests =====

    #[test]
    fn test_host_call() {
        let expr = Expression::HostCall {
            function_name: "hstringEquals".into(),
            args: vec![
                Expression::FieldAccess {
                    object: Box::new(Expression::Param { index: 0 }),
                    class_name: "Employee".into(),
                    field_name: "skill".into(),
                },
                Expression::FieldAccess {
                    object: Box::new(Expression::Param { index: 1 }),
                    class_name: "Shift".into(),
                    field_name: "requiredSkill".into(),
                },
            ],
        };

        match expr {
            Expression::HostCall {
                function_name,
                args,
            } => {
                assert_eq!(function_name, "hstringEquals");
                assert_eq!(args.len(), 2);
            }
            _ => panic!("Expected HostCall"),
        }
    }

    #[test]
    fn test_serialize_host_call() {
        let expr = Expression::HostCall {
            function_name: "hstringEquals".into(),
            args: vec![
                Expression::IntLiteral { value: 1 },
                Expression::IntLiteral { value: 2 },
            ],
        };

        let json = serde_json::to_string(&expr).unwrap();
        let deserialized: Expression = serde_json::from_str(&json).unwrap();
        assert_eq!(expr, deserialized);
    }

    #[test]
    fn test_host_call_with_no_args() {
        let expr = Expression::HostCall {
            function_name: "hnewList".into(),
            args: vec![],
        };

        let json = serde_json::to_string(&expr).unwrap();
        let deserialized: Expression = serde_json::from_str(&json).unwrap();
        assert_eq!(expr, deserialized);
    }

    #[test]
    fn test_complex_host_call_expression() {
        // Build: hstringEquals(employee.skill, shift.requiredSkill)
        // nested in a logical expression: employee != null && hstringEquals(...)
        let expr = Expression::And {
            left: Box::new(Expression::IsNotNull {
                operand: Box::new(Expression::FieldAccess {
                    object: Box::new(Expression::Param { index: 0 }),
                    class_name: "Shift".into(),
                    field_name: "employee".into(),
                }),
            }),
            right: Box::new(Expression::HostCall {
                function_name: "hstringEquals".into(),
                args: vec![
                    Expression::FieldAccess {
                        object: Box::new(Expression::Param { index: 0 }),
                        class_name: "Employee".into(),
                        field_name: "skill".into(),
                    },
                    Expression::FieldAccess {
                        object: Box::new(Expression::Param { index: 1 }),
                        class_name: "Shift".into(),
                        field_name: "requiredSkill".into(),
                    },
                ],
            }),
        };

        // Serialize and deserialize
        let json = serde_json::to_string(&expr).unwrap();
        let deserialized: Expression = serde_json::from_str(&json).unwrap();
        assert_eq!(expr, deserialized);
    }

    #[test]
    fn test_list_contains() {
        let expr = Expression::ListContains {
            list: Box::new(Expression::FieldAccess {
                object: Box::new(Expression::Param { index: 0 }),
                class_name: "Employee".into(),
                field_name: "skills".into(),
            }),
            element: Box::new(Expression::FieldAccess {
                object: Box::new(Expression::Param { index: 1 }),
                class_name: "Shift".into(),
                field_name: "requiredSkill".into(),
            }),
        };

        match &expr {
            Expression::ListContains { list, element } => {
                assert!(matches!(
                    **list,
                    Expression::FieldAccess {
                        field_name: ref name,
                        ..
                    } if name == "skills"
                ));
                assert!(matches!(
                    **element,
                    Expression::FieldAccess {
                        field_name: ref name,
                        ..
                    } if name == "requiredSkill"
                ));
            }
            _ => panic!("Expected ListContains expression"),
        }

        // Serialize and deserialize
        let json = serde_json::to_string(&expr).unwrap();
        let deserialized: Expression = serde_json::from_str(&json).unwrap();
        assert_eq!(expr, deserialized);
    }
}
