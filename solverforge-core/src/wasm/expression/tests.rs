use super::Expression;

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
    let expr = Expression::Ne {
        left: Box::new(Expression::FieldAccess {
            object: Box::new(Expression::Param { index: 0 }),
            class_name: "Shift".into(),
            field_name: "employee".into(),
        }),
        right: Box::new(Expression::Null),
    };

    let json = serde_json::to_string(&expr).unwrap();
    let deserialized: Expression = serde_json::from_str(&json).unwrap();
    assert_eq!(expr, deserialized);
}

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

    let json = serde_json::to_string(&expr).unwrap();
    let deserialized: Expression = serde_json::from_str(&json).unwrap();
    assert_eq!(expr, deserialized);
}

#[test]
fn test_time_calculation_expression() {
    let expr = Expression::Div {
        left: Box::new(Expression::FieldAccess {
            object: Box::new(Expression::Param { index: 0 }),
            class_name: "Shift".into(),
            field_name: "start".into(),
        }),
        right: Box::new(Expression::IntLiteral { value: 24 }),
    };

    let json = serde_json::to_string(&expr).unwrap();
    let deserialized: Expression = serde_json::from_str(&json).unwrap();
    assert_eq!(expr, deserialized);
}

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

    let json = serde_json::to_string(&expr).unwrap();
    let deserialized: Expression = serde_json::from_str(&json).unwrap();
    assert_eq!(expr, deserialized);
}

// ===== substitute_param Tests =====

#[test]
fn test_substitute_param_simple() {
    let expr = Expression::Param { index: 0 };
    let substitute = Expression::IntLiteral { value: 42 };

    let result = expr.substitute_param(0, &substitute);
    assert_eq!(result, Expression::IntLiteral { value: 42 });
}

#[test]
fn test_substitute_param_no_match() {
    let expr = Expression::Param { index: 1 };
    let substitute = Expression::IntLiteral { value: 42 };

    let result = expr.substitute_param(0, &substitute);
    assert_eq!(result, Expression::Param { index: 1 });
}

#[test]
fn test_substitute_param_in_field_access() {
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
            match *condition {
                Expression::Gt { left, .. } => {
                    assert_eq!(*left, Expression::IntLiteral { value: 42 });
                }
                _ => panic!("Expected Gt"),
            }
            assert_eq!(*then_branch, Expression::IntLiteral { value: 42 });
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
            match *collection {
                Expression::FieldAccess { object, .. } => match *object {
                    Expression::FieldAccess { object: inner, .. } => {
                        assert_eq!(*inner, Expression::Param { index: 2 });
                    }
                    _ => panic!("Expected nested FieldAccess"),
                },
                _ => panic!("Expected FieldAccess"),
            }
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
