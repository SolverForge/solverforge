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
    /// Integer literal (i64) - compiles to i32 in WASM
    IntLiteral { value: i64 },

    /// 64-bit integer literal - compiles directly to i64 in WASM
    Int64Literal { value: i64 },

    /// Float literal (f64)
    FloatLiteral { value: f64 },

    /// String literal
    /// Used for string comparisons. At WASM generation time, the string is stored
    /// in a data segment and a pointer to it is used for comparison via hstringEquals.
    StringLiteral { value: String },

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

    // ===== i64 Comparisons =====
    /// Equal comparison for i64
    Eq64 {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Not equal comparison for i64
    Ne64 {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Less than comparison for i64
    Lt64 {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Less than or equal comparison for i64
    Le64 {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Greater than comparison for i64
    Gt64 {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Greater than or equal comparison for i64
    Ge64 {
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

    // ===== i64 Arithmetic Operations =====
    /// Addition for i64
    Add64 {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Subtraction for i64
    Sub64 {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Multiplication for i64
    Mul64 {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Division for i64
    Div64 {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    // ===== Float Arithmetic Operations =====
    /// Float addition (f64)
    FloatAdd {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Float subtraction (f64)
    FloatSub {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Float multiplication (f64)
    FloatMul {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Float division (f64)
    FloatDiv {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    // ===== Math Functions =====
    /// Square root (WASM f64.sqrt intrinsic)
    Sqrt { operand: Box<Expression> },

    /// Absolute value for floats (WASM f64.abs intrinsic)
    FloatAbs { operand: Box<Expression> },

    /// Round to nearest integer (WASM f64.nearest intrinsic)
    Round { operand: Box<Expression> },

    /// Floor (WASM f64.floor intrinsic)
    Floor { operand: Box<Expression> },

    /// Ceiling (WASM f64.ceil intrinsic)
    Ceil { operand: Box<Expression> },

    /// Sine (host call)
    Sin { operand: Box<Expression> },

    /// Cosine (host call)
    Cos { operand: Box<Expression> },

    /// Arc sine (host call)
    Asin { operand: Box<Expression> },

    /// Arc cosine (host call)
    Acos { operand: Box<Expression> },

    /// Arc tangent (host call)
    Atan { operand: Box<Expression> },

    /// Arc tangent of y/x (host call)
    Atan2 {
        y: Box<Expression>,
        x: Box<Expression>,
    },

    /// Convert degrees to radians
    Radians { operand: Box<Expression> },

    /// Convert int to float
    IntToFloat { operand: Box<Expression> },

    /// Convert float to int (truncating)
    FloatToInt { operand: Box<Expression> },

    // ===== List Operations =====
    /// Check if a list contains an element
    /// Example: list.contains(element)
    ListContains {
        list: Box<Expression>,
        element: Box<Expression>,
    },

    /// Get the length of a collection
    /// Example: len(vehicle.visits)
    Length { collection: Box<Expression> },

    /// Sum of field values over a collection
    /// Example: sum(item.demand for item in vehicle.visits)
    ///
    /// # Fields
    /// * `collection` - The collection to iterate over (e.g., vehicle.visits)
    /// * `item_var_name` - Name of the loop variable (e.g., "item", "visit")
    /// * `item_param_index` - Parameter index of the loop variable in the accumulator expression
    /// * `item_class_name` - The type of items in the collection (e.g., "Visit")
    /// * `accumulator_expr` - Expression to sum (e.g., Param(item_param_index).demand)
    ///
    /// The accumulator_expr should reference the loop variable by its parameter index.
    /// The WASM generator replaces references to item_param_index with loads from the
    /// loop element local variable and uses item_class_name for field lookups.
    Sum {
        collection: Box<Expression>,
        item_var_name: String,
        item_param_index: u32,
        item_class_name: String,
        accumulator_expr: Box<Expression>,
    },

    /// Access the last element of a collection
    ///
    /// Used for post-loop terms in accumulation patterns where mutable tracking
    /// variables need to reference the final element's value.
    ///
    /// # Fields
    /// * `collection` - The collection expression (e.g., self.visits)
    /// * `item_class_name` - The class name of items in the collection
    LastElement {
        collection: Box<Expression>,
        item_class_name: String,
    },

    // ===== Host Function Calls =====
    /// Call a host-provided function
    /// Example: hstringEquals(left, right)
    HostCall {
        function_name: String,
        args: Vec<Expression>,
    },

    // ===== Conditional =====
    /// If-then-else conditional expression (produces i32)
    /// Example: if condition { then_branch } else { else_branch }
    IfThenElse {
        condition: Box<Expression>,
        then_branch: Box<Expression>,
        else_branch: Box<Expression>,
    },

    /// If-then-else conditional expression (produces i64)
    /// Used when branches are i64 values (e.g., datetime arithmetic)
    IfThenElse64 {
        condition: Box<Expression>,
        then_branch: Box<Expression>,
        else_branch: Box<Expression>,
    },

    // ===== Type Conversions =====
    /// Wrap i64 to i32 (truncate)
    I64ToI32 { operand: Box<Expression> },

    /// Extend i32 to i64 (signed)
    I32ToI64 { operand: Box<Expression> },
}

impl Expression {
    /// Substitute all occurrences of a parameter with a replacement expression.
    ///
    /// This is used for method inlining: when inlining `obj.method()`, we replace
    /// `Param(0)` (self) with `obj`, and other parameters with their call arguments.
    ///
    /// # Example
    /// ```
    /// # use solverforge_core::wasm::Expression;
    /// // Method body: self.field + self.other
    /// let body = Expression::Add {
    ///     left: Box::new(Expression::FieldAccess {
    ///         object: Box::new(Expression::Param { index: 0 }),
    ///         class_name: "Obj".into(),
    ///         field_name: "field".into(),
    ///     }),
    ///     right: Box::new(Expression::FieldAccess {
    ///         object: Box::new(Expression::Param { index: 0 }),
    ///         class_name: "Obj".into(),
    ///         field_name: "other".into(),
    ///     }),
    /// };
    ///
    /// // Substitute Param(0) with a specific object reference
    /// let obj = Expression::FieldAccess {
    ///     object: Box::new(Expression::Param { index: 1 }),
    ///     class_name: "Container".into(),
    ///     field_name: "item".into(),
    /// };
    ///
    /// let inlined = body.substitute_param(0, &obj);
    /// // Result: container.item.field + container.item.other
    /// ```
    pub fn substitute_param(self, from_index: u32, substitute: &Expression) -> Expression {
        // Macros to reduce repetition
        macro_rules! sub {
            ($e:expr) => {
                Box::new((*$e).substitute_param(from_index, substitute))
            };
        }

        macro_rules! binary {
            ($variant:ident, $left:expr, $right:expr) => {
                Expression::$variant {
                    left: sub!($left),
                    right: sub!($right),
                }
            };
        }

        macro_rules! unary {
            ($variant:ident, $operand:expr) => {
                Expression::$variant {
                    operand: sub!($operand),
                }
            };
        }

        match self {
            // Parameter substitution - the core operation
            Expression::Param { index } if index == from_index => substitute.clone(),
            Expression::Param { index } => Expression::Param { index },

            // Field access
            Expression::FieldAccess {
                object,
                class_name,
                field_name,
            } => Expression::FieldAccess {
                object: sub!(object),
                class_name,
                field_name,
            },

            // Comparisons (i32)
            Expression::Eq { left, right } => binary!(Eq, left, right),
            Expression::Ne { left, right } => binary!(Ne, left, right),
            Expression::Lt { left, right } => binary!(Lt, left, right),
            Expression::Le { left, right } => binary!(Le, left, right),
            Expression::Gt { left, right } => binary!(Gt, left, right),
            Expression::Ge { left, right } => binary!(Ge, left, right),

            // Comparisons (i64)
            Expression::Eq64 { left, right } => binary!(Eq64, left, right),
            Expression::Ne64 { left, right } => binary!(Ne64, left, right),
            Expression::Lt64 { left, right } => binary!(Lt64, left, right),
            Expression::Le64 { left, right } => binary!(Le64, left, right),
            Expression::Gt64 { left, right } => binary!(Gt64, left, right),
            Expression::Ge64 { left, right } => binary!(Ge64, left, right),

            // Arithmetic (i32)
            Expression::Add { left, right } => binary!(Add, left, right),
            Expression::Sub { left, right } => binary!(Sub, left, right),
            Expression::Mul { left, right } => binary!(Mul, left, right),
            Expression::Div { left, right } => binary!(Div, left, right),

            // Arithmetic (i64)
            Expression::Add64 { left, right } => binary!(Add64, left, right),
            Expression::Sub64 { left, right } => binary!(Sub64, left, right),
            Expression::Mul64 { left, right } => binary!(Mul64, left, right),
            Expression::Div64 { left, right } => binary!(Div64, left, right),

            // Arithmetic (f64)
            Expression::FloatAdd { left, right } => binary!(FloatAdd, left, right),
            Expression::FloatSub { left, right } => binary!(FloatSub, left, right),
            Expression::FloatMul { left, right } => binary!(FloatMul, left, right),
            Expression::FloatDiv { left, right } => binary!(FloatDiv, left, right),

            // Math functions - unary
            Expression::Sqrt { operand } => unary!(Sqrt, operand),
            Expression::FloatAbs { operand } => unary!(FloatAbs, operand),
            Expression::Round { operand } => unary!(Round, operand),
            Expression::Floor { operand } => unary!(Floor, operand),
            Expression::Ceil { operand } => unary!(Ceil, operand),
            Expression::Sin { operand } => unary!(Sin, operand),
            Expression::Cos { operand } => unary!(Cos, operand),
            Expression::Asin { operand } => unary!(Asin, operand),
            Expression::Acos { operand } => unary!(Acos, operand),
            Expression::Atan { operand } => unary!(Atan, operand),
            Expression::Radians { operand } => unary!(Radians, operand),
            Expression::IntToFloat { operand } => unary!(IntToFloat, operand),
            Expression::FloatToInt { operand } => unary!(FloatToInt, operand),

            // Math functions - binary
            Expression::Atan2 { y, x } => Expression::Atan2 {
                y: sub!(y),
                x: sub!(x),
            },

            // Logical operations
            Expression::And { left, right } => binary!(And, left, right),
            Expression::Or { left, right } => binary!(Or, left, right),
            Expression::Not { operand } => unary!(Not, operand),
            Expression::IsNull { operand } => unary!(IsNull, operand),
            Expression::IsNotNull { operand } => unary!(IsNotNull, operand),

            // Host calls
            Expression::HostCall {
                function_name,
                args,
            } => Expression::HostCall {
                function_name,
                args: args
                    .into_iter()
                    .map(|arg| arg.substitute_param(from_index, substitute))
                    .collect(),
            },

            // List operations
            Expression::ListContains { list, element } => Expression::ListContains {
                list: sub!(list),
                element: sub!(element),
            },
            Expression::Length { collection } => Expression::Length {
                collection: sub!(collection),
            },

            // Sum with index adjustment
            Expression::Sum {
                collection,
                item_var_name,
                item_param_index,
                item_class_name,
                accumulator_expr,
            } => {
                // Adjust item_param_index if we're substituting a lower index
                let new_index = if from_index < item_param_index {
                    item_param_index - 1
                } else {
                    item_param_index
                };

                Expression::Sum {
                    collection: sub!(collection),
                    item_var_name,
                    item_param_index: new_index,
                    item_class_name,
                    accumulator_expr: sub!(accumulator_expr),
                }
            }

            Expression::LastElement {
                collection,
                item_class_name,
            } => Expression::LastElement {
                collection: sub!(collection),
                item_class_name,
            },

            // Conditional
            Expression::IfThenElse {
                condition,
                then_branch,
                else_branch,
            } => Expression::IfThenElse {
                condition: sub!(condition),
                then_branch: sub!(then_branch),
                else_branch: sub!(else_branch),
            },
            Expression::IfThenElse64 {
                condition,
                then_branch,
                else_branch,
            } => Expression::IfThenElse64 {
                condition: sub!(condition),
                then_branch: sub!(then_branch),
                else_branch: sub!(else_branch),
            },

            // Type conversions
            Expression::I64ToI32 { operand } => unary!(I64ToI32, operand),
            Expression::I32ToI64 { operand } => unary!(I32ToI64, operand),

            // Literals - no params, return as-is
            Expression::Null
            | Expression::BoolLiteral { .. }
            | Expression::IntLiteral { .. }
            | Expression::Int64Literal { .. }
            | Expression::FloatLiteral { .. }
            | Expression::StringLiteral { .. } => self,
        }
    }
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
    fn test_float_literal() {
        let expr = Expression::FloatLiteral { value: 3.14 };
        assert_eq!(expr, Expression::FloatLiteral { value: 3.14 });
    }

    #[test]
    fn test_string_literal() {
        let expr = Expression::StringLiteral {
            value: "active".into(),
        };
        assert_eq!(
            expr,
            Expression::StringLiteral {
                value: "active".into()
            }
        );
    }

    #[test]
    fn test_serialize_string_literal() {
        let expr = Expression::StringLiteral {
            value: "hello".into(),
        };
        let json = serde_json::to_string(&expr).unwrap();
        assert!(json.contains("\"kind\":\"StringLiteral\""));
        assert!(json.contains("\"value\":\"hello\""));

        let deserialized: Expression = serde_json::from_str(&json).unwrap();
        assert_eq!(expr, deserialized);
    }

    #[test]
    fn test_serialize_float_literal() {
        let expr = Expression::FloatLiteral { value: 2.5 };
        let json = serde_json::to_string(&expr).unwrap();
        assert!(json.contains("\"kind\":\"FloatLiteral\""));

        let deserialized: Expression = serde_json::from_str(&json).unwrap();
        assert_eq!(expr, deserialized);
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
                right: Box::new(Expression::IntLiteral { value: 42 }),
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

    // ===== substitute_param Tests =====

    #[test]
    fn test_substitute_param_simple() {
        // Param(0) -> substitute
        let expr = Expression::Param { index: 0 };
        let substitute = Expression::IntLiteral { value: 42 };

        let result = expr.substitute_param(0, &substitute);
        assert_eq!(result, Expression::IntLiteral { value: 42 });
    }

    #[test]
    fn test_substitute_param_no_match() {
        // Param(1) should not be substituted when replacing Param(0)
        let expr = Expression::Param { index: 1 };
        let substitute = Expression::IntLiteral { value: 42 };

        let result = expr.substitute_param(0, &substitute);
        assert_eq!(result, Expression::Param { index: 1 });
    }

    #[test]
    fn test_substitute_param_in_field_access() {
        // Param(0).field -> substitute.field
        let expr = Expression::FieldAccess {
            object: Box::new(Expression::Param { index: 0 }),
            class_name: "Employee".into(),
            field_name: "name".into(),
        };
        let substitute = Expression::FieldAccess {
            object: Box::new(Expression::Param { index: 1 }),
            class_name: "Container".into(),
            field_name: "item".into(),
        };

        let result = expr.substitute_param(0, &substitute);

        match result {
            Expression::FieldAccess {
                object, field_name, ..
            } => {
                assert_eq!(field_name, "name");
                // The object should now be the substitute
                match *object {
                    Expression::FieldAccess {
                        field_name: inner_name,
                        ..
                    } => {
                        assert_eq!(inner_name, "item");
                    }
                    _ => panic!("Expected FieldAccess"),
                }
            }
            _ => panic!("Expected FieldAccess"),
        }
    }

    #[test]
    fn test_substitute_param_in_binary_op() {
        // Param(0) + Param(1) with Param(0) -> Literal(10)
        let expr = Expression::Add {
            left: Box::new(Expression::Param { index: 0 }),
            right: Box::new(Expression::Param { index: 1 }),
        };
        let substitute = Expression::IntLiteral { value: 10 };

        let result = expr.substitute_param(0, &substitute);

        match result {
            Expression::Add { left, right } => {
                assert_eq!(*left, Expression::IntLiteral { value: 10 });
                assert_eq!(*right, Expression::Param { index: 1 });
            }
            _ => panic!("Expected Add"),
        }
    }

    #[test]
    fn test_substitute_param_in_comparison() {
        // Param(0).value > Param(1).value
        let expr = Expression::Gt {
            left: Box::new(Expression::FieldAccess {
                object: Box::new(Expression::Param { index: 0 }),
                class_name: "A".into(),
                field_name: "value".into(),
            }),
            right: Box::new(Expression::FieldAccess {
                object: Box::new(Expression::Param { index: 1 }),
                class_name: "B".into(),
                field_name: "value".into(),
            }),
        };
        let substitute = Expression::IntLiteral { value: 5 };

        let result = expr.substitute_param(0, &substitute);

        match result {
            Expression::Gt { left, right } => {
                match *left {
                    Expression::FieldAccess { object, .. } => {
                        assert_eq!(*object, Expression::IntLiteral { value: 5 });
                    }
                    _ => panic!("Expected FieldAccess"),
                }
                // Right side should still have Param(1)
                match *right {
                    Expression::FieldAccess { object, .. } => {
                        assert_eq!(*object, Expression::Param { index: 1 });
                    }
                    _ => panic!("Expected FieldAccess"),
                }
            }
            _ => panic!("Expected Gt"),
        }
    }

    #[test]
    fn test_substitute_param_literal_unchanged() {
        let expr = Expression::IntLiteral { value: 100 };
        let substitute = Expression::IntLiteral { value: 999 };

        let result = expr.substitute_param(0, &substitute);
        assert_eq!(result, Expression::IntLiteral { value: 100 });
    }

    #[test]
    fn test_substitute_param_in_if_then_else() {
        // if Param(0) > 0 then Param(0) else Param(1)
        let expr = Expression::IfThenElse {
            condition: Box::new(Expression::Gt {
                left: Box::new(Expression::Param { index: 0 }),
                right: Box::new(Expression::IntLiteral { value: 0 }),
            }),
            then_branch: Box::new(Expression::Param { index: 0 }),
            else_branch: Box::new(Expression::Param { index: 1 }),
        };
        let substitute = Expression::IntLiteral { value: 42 };

        let result = expr.substitute_param(0, &substitute);

        match result {
            Expression::IfThenElse {
                condition,
                then_branch,
                else_branch,
            } => {
                // condition: 42 > 0
                match *condition {
                    Expression::Gt { left, .. } => {
                        assert_eq!(*left, Expression::IntLiteral { value: 42 });
                    }
                    _ => panic!("Expected Gt"),
                }
                // then: 42
                assert_eq!(*then_branch, Expression::IntLiteral { value: 42 });
                // else: still Param(1)
                assert_eq!(*else_branch, Expression::Param { index: 1 });
            }
            _ => panic!("Expected IfThenElse"),
        }
    }

    #[test]
    fn test_substitute_param_in_host_call() {
        let expr = Expression::HostCall {
            function_name: "hstringEquals".into(),
            args: vec![
                Expression::Param { index: 0 },
                Expression::Param { index: 1 },
            ],
        };
        let substitute = Expression::StringLiteral {
            value: "test".into(),
        };

        let result = expr.substitute_param(0, &substitute);

        match result {
            Expression::HostCall { args, .. } => {
                assert_eq!(args.len(), 2);
                assert_eq!(
                    args[0],
                    Expression::StringLiteral {
                        value: "test".into()
                    }
                );
                assert_eq!(args[1], Expression::Param { index: 1 });
            }
            _ => panic!("Expected HostCall"),
        }
    }

    #[test]
    fn test_substitute_param_in_sum() {
        // Sum over Param(0).items with accumulator using Param(1)
        let expr = Expression::Sum {
            collection: Box::new(Expression::FieldAccess {
                object: Box::new(Expression::Param { index: 0 }),
                class_name: "Vehicle".into(),
                field_name: "visits".into(),
            }),
            item_var_name: "visit".into(),
            item_param_index: 1,
            item_class_name: "Visit".into(),
            accumulator_expr: Box::new(Expression::FieldAccess {
                object: Box::new(Expression::Param { index: 1 }),
                class_name: "Visit".into(),
                field_name: "demand".into(),
            }),
        };

        let substitute = Expression::FieldAccess {
            object: Box::new(Expression::Param { index: 2 }),
            class_name: "Solution".into(),
            field_name: "vehicle".into(),
        };

        let result = expr.substitute_param(0, &substitute);

        match result {
            Expression::Sum {
                collection,
                item_param_index,
                ..
            } => {
                // Collection should now reference Param(2).vehicle.visits
                match *collection {
                    Expression::FieldAccess { object, .. } => match *object {
                        Expression::FieldAccess { object: inner, .. } => {
                            assert_eq!(*inner, Expression::Param { index: 2 });
                        }
                        _ => panic!("Expected nested FieldAccess"),
                    },
                    _ => panic!("Expected FieldAccess"),
                }
                // item_param_index should be decremented (0 < 1, so 1 -> 0)
                assert_eq!(item_param_index, 0);
            }
            _ => panic!("Expected Sum"),
        }
    }

    #[test]
    fn test_substitute_param_unary_ops() {
        let expr = Expression::Not {
            operand: Box::new(Expression::IsNull {
                operand: Box::new(Expression::Param { index: 0 }),
            }),
        };
        let substitute = Expression::FieldAccess {
            object: Box::new(Expression::Param { index: 1 }),
            class_name: "X".into(),
            field_name: "y".into(),
        };

        let result = expr.substitute_param(0, &substitute);

        match result {
            Expression::Not { operand } => match *operand {
                Expression::IsNull { operand: inner } => match *inner {
                    Expression::FieldAccess { field_name, .. } => {
                        assert_eq!(field_name, "y");
                    }
                    _ => panic!("Expected FieldAccess"),
                },
                _ => panic!("Expected IsNull"),
            },
            _ => panic!("Expected Not"),
        }
    }
}
