//! Tests for lambda analyzer.
//!
//! These tests verify lambda source parsing by calling analyze_lambda_source directly.

use super::*;
use solverforge_core::wasm::WasmFieldType;

#[test]
fn test_generate_lambda_name_unique() {
    let name1 = generate_lambda_name("test");
    let name2 = generate_lambda_name("test");
    assert_ne!(name1, name2);
    assert!(name1.starts_with("test_"));
    assert!(name2.starts_with("test_"));
}

#[test]
fn test_analyze_simple_field_access() {
    pyo3::Python::initialize();
    Python::attach(|py| {
        let expr = analyze_lambda_source(py, "lambda x: x.timeslot", 1, "Lesson").unwrap();
        match &expr {
            Expression::FieldAccess {
                field_name,
                class_name,
                ..
            } => {
                assert_eq!(field_name, "timeslot");
                assert_eq!(class_name, "Lesson");
            }
            _ => panic!("Expected FieldAccess, got {:?}", expr),
        }
    });
}

#[test]
fn test_analyze_is_not_none() {
    pyo3::Python::initialize();
    Python::attach(|py| {
        let expr = analyze_lambda_source(py, "lambda x: x.room is not None", 1, "Lesson").unwrap();
        match &expr {
            Expression::IsNotNull { operand } => match operand.as_ref() {
                Expression::FieldAccess { field_name, .. } => {
                    assert_eq!(field_name, "room");
                }
                _ => panic!("Expected FieldAccess inside IsNotNull"),
            },
            _ => panic!("Expected IsNotNull, got {:?}", expr),
        }
    });
}

#[test]
fn test_analyze_is_none() {
    pyo3::Python::initialize();
    Python::attach(|py| {
        let expr = analyze_lambda_source(py, "lambda x: x.room is None", 1, "Lesson").unwrap();
        match &expr {
            Expression::IsNull { operand } => match operand.as_ref() {
                Expression::FieldAccess { field_name, .. } => {
                    assert_eq!(field_name, "room");
                }
                _ => panic!("Expected FieldAccess inside IsNull"),
            },
            _ => panic!("Expected IsNull, got {:?}", expr),
        }
    });
}

#[test]
fn test_analyze_comparison_gt() {
    pyo3::Python::initialize();
    Python::attach(|py| {
        let expr = analyze_lambda_source(py, "lambda x: x.count > 5", 1, "Entity").unwrap();
        match &expr {
            Expression::Gt { left, right } => {
                assert!(
                    matches!(left.as_ref(), Expression::FieldAccess { field_name, .. } if field_name == "count")
                );
                assert!(matches!(
                    right.as_ref(),
                    Expression::IntLiteral { value: 5 }
                ));
            }
            _ => panic!("Expected Gt, got {:?}", expr),
        }
    });
}

#[test]
fn test_analyze_comparison_eq() {
    pyo3::Python::initialize();
    Python::attach(|py| {
        let expr = analyze_lambda_source(py, "lambda x: x.value == 10", 1, "Entity").unwrap();
        match &expr {
            Expression::Eq { left, right } => {
                assert!(
                    matches!(left.as_ref(), Expression::FieldAccess { field_name, .. } if field_name == "value")
                );
                assert!(matches!(
                    right.as_ref(),
                    Expression::IntLiteral { value: 10 }
                ));
            }
            _ => panic!("Expected Eq, got {:?}", expr),
        }
    });
}

#[test]
fn test_analyze_and_expression() {
    pyo3::Python::initialize();
    Python::attach(|py| {
        let expr = analyze_lambda_source(py, "lambda x: x.a and x.b", 1, "Entity").unwrap();
        match &expr {
            Expression::And { left, right } => {
                assert!(
                    matches!(left.as_ref(), Expression::FieldAccess { field_name, .. } if field_name == "a")
                );
                assert!(
                    matches!(right.as_ref(), Expression::FieldAccess { field_name, .. } if field_name == "b")
                );
            }
            _ => panic!("Expected And, got {:?}", expr),
        }
    });
}

#[test]
fn test_analyze_or_expression() {
    pyo3::Python::initialize();
    Python::attach(|py| {
        let expr = analyze_lambda_source(py, "lambda x: x.a or x.b", 1, "Entity").unwrap();
        match &expr {
            Expression::Or { left, right } => {
                assert!(
                    matches!(left.as_ref(), Expression::FieldAccess { field_name, .. } if field_name == "a")
                );
                assert!(
                    matches!(right.as_ref(), Expression::FieldAccess { field_name, .. } if field_name == "b")
                );
            }
            _ => panic!("Expected Or, got {:?}", expr),
        }
    });
}

#[test]
fn test_analyze_not_expression() {
    pyo3::Python::initialize();
    Python::attach(|py| {
        let expr = analyze_lambda_source(py, "lambda x: not x.a", 1, "Entity").unwrap();
        match &expr {
            Expression::Not { operand } => {
                assert!(
                    matches!(operand.as_ref(), Expression::FieldAccess { field_name, .. } if field_name == "a")
                );
            }
            _ => panic!("Expected Not, got {:?}", expr),
        }
    });
}

#[test]
fn test_analyze_nested_field_access() {
    pyo3::Python::initialize();
    Python::attach(|py| {
        let expr = analyze_lambda_source(py, "lambda x: x.timeslot.day", 1, "Lesson").unwrap();
        match &expr {
            Expression::FieldAccess {
                object, field_name, ..
            } => {
                assert_eq!(field_name, "day");
                match object.as_ref() {
                    Expression::FieldAccess { field_name, .. } => {
                        assert_eq!(field_name, "timeslot");
                    }
                    _ => panic!("Expected nested FieldAccess"),
                }
            }
            _ => panic!("Expected FieldAccess, got {:?}", expr),
        }
    });
}

#[test]
fn test_analyze_arithmetic_add() {
    pyo3::Python::initialize();
    Python::attach(|py| {
        let expr = analyze_lambda_source(py, "lambda x: x.value + 10", 1, "Entity").unwrap();
        match &expr {
            Expression::Add { left, right } => {
                assert!(
                    matches!(left.as_ref(), Expression::FieldAccess { field_name, .. } if field_name == "value")
                );
                assert!(matches!(
                    right.as_ref(),
                    Expression::IntLiteral { value: 10 }
                ));
            }
            _ => panic!("Expected Add, got {:?}", expr),
        }
    });
}

#[test]
fn test_analyze_arithmetic_sub() {
    pyo3::Python::initialize();
    Python::attach(|py| {
        let expr = analyze_lambda_source(py, "lambda x: x.value - 5", 1, "Entity").unwrap();
        match &expr {
            Expression::Sub { left, right } => {
                assert!(
                    matches!(left.as_ref(), Expression::FieldAccess { field_name, .. } if field_name == "value")
                );
                assert!(matches!(
                    right.as_ref(),
                    Expression::IntLiteral { value: 5 }
                ));
            }
            _ => panic!("Expected Sub, got {:?}", expr),
        }
    });
}

#[test]
fn test_analyze_arithmetic_mul() {
    pyo3::Python::initialize();
    Python::attach(|py| {
        let expr = analyze_lambda_source(py, "lambda x: x.value * 2", 1, "Entity").unwrap();
        match &expr {
            Expression::Mul { left, right } => {
                assert!(
                    matches!(left.as_ref(), Expression::FieldAccess { field_name, .. } if field_name == "value")
                );
                assert!(matches!(
                    right.as_ref(),
                    Expression::IntLiteral { value: 2 }
                ));
            }
            _ => panic!("Expected Mul, got {:?}", expr),
        }
    });
}

#[test]
fn test_analyze_arithmetic_div() {
    pyo3::Python::initialize();
    Python::attach(|py| {
        // Python `/` is true division, produces FloatDiv
        let expr = analyze_lambda_source(py, "lambda x: x.value / 2", 1, "Entity").unwrap();
        match &expr {
            Expression::FloatDiv { left, right } => {
                assert!(
                    matches!(left.as_ref(), Expression::FieldAccess { field_name, .. } if field_name == "value")
                );
                assert!(matches!(
                    right.as_ref(),
                    Expression::FloatLiteral { value } if (*value - 2.0).abs() < f64::EPSILON
                ));
            }
            _ => panic!("Expected FloatDiv, got {:?}", expr),
        }
    });
}

#[test]
fn test_analyze_bi_lambda() {
    pyo3::Python::initialize();
    Python::attach(|py| {
        let expr = analyze_lambda_source(py, "lambda a, b: a.room == b.room", 2, "Lesson").unwrap();
        match &expr {
            Expression::Eq { left, right } => {
                // left should be Param(0).room
                match left.as_ref() {
                    Expression::FieldAccess {
                        object, field_name, ..
                    } => {
                        assert_eq!(field_name, "room");
                        assert!(matches!(object.as_ref(), Expression::Param { index: 0 }));
                    }
                    _ => panic!("Expected FieldAccess on left"),
                }
                // right should be Param(1).room
                match right.as_ref() {
                    Expression::FieldAccess {
                        object, field_name, ..
                    } => {
                        assert_eq!(field_name, "room");
                        assert!(matches!(object.as_ref(), Expression::Param { index: 1 }));
                    }
                    _ => panic!("Expected FieldAccess on right"),
                }
            }
            _ => panic!("Expected Eq, got {:?}", expr),
        }
    });
}

#[test]
fn test_analyze_bi_lambda_direct_param_add() {
    pyo3::Python::initialize();
    Python::attach(|py| {
        let expr =
            analyze_lambda_source(py, "lambda a, b: a.value + b.value", 2, "Entity").unwrap();
        match &expr {
            Expression::Add { left, right } => {
                match left.as_ref() {
                    Expression::FieldAccess {
                        object, field_name, ..
                    } => {
                        assert_eq!(field_name, "value");
                        assert!(matches!(object.as_ref(), Expression::Param { index: 0 }));
                    }
                    _ => panic!("Expected FieldAccess on left"),
                }
                match right.as_ref() {
                    Expression::FieldAccess {
                        object, field_name, ..
                    } => {
                        assert_eq!(field_name, "value");
                        assert!(matches!(object.as_ref(), Expression::Param { index: 1 }));
                    }
                    _ => panic!("Expected FieldAccess on right"),
                }
            }
            _ => panic!("Expected Add, got {:?}", expr),
        }
    });
}

#[test]
fn test_analyze_tri_lambda_arithmetic() {
    pyo3::Python::initialize();
    Python::attach(|py| {
        let expr = analyze_lambda_source(
            py,
            "lambda a, b, c: a.value + b.value + c.value",
            3,
            "Entity",
        )
        .unwrap();
        // Should be Add(Add(a.value, b.value), c.value)
        match &expr {
            Expression::Add { left, right } => {
                // right should be c.value (Param 2)
                match right.as_ref() {
                    Expression::FieldAccess {
                        object, field_name, ..
                    } => {
                        assert_eq!(field_name, "value");
                        assert!(matches!(object.as_ref(), Expression::Param { index: 2 }));
                    }
                    _ => panic!("Expected FieldAccess on right"),
                }
                // left should be Add(a.value, b.value)
                match left.as_ref() {
                    Expression::Add {
                        left: ll,
                        right: lr,
                    } => {
                        match ll.as_ref() {
                            Expression::FieldAccess { object, .. } => {
                                assert!(matches!(object.as_ref(), Expression::Param { index: 0 }));
                            }
                            _ => panic!("Expected FieldAccess"),
                        }
                        match lr.as_ref() {
                            Expression::FieldAccess { object, .. } => {
                                assert!(matches!(object.as_ref(), Expression::Param { index: 1 }));
                            }
                            _ => panic!("Expected FieldAccess"),
                        }
                    }
                    _ => panic!("Expected nested Add on left"),
                }
            }
            _ => panic!("Expected Add, got {:?}", expr),
        }
    });
}

#[test]
fn test_substitute_param_replaces_correctly() {
    let expr = Expression::Add {
        left: Box::new(Expression::Param { index: 0 }),
        right: Box::new(Expression::IntLiteral { value: 5 }),
    };

    let substituted = substitute_param(
        expr,
        0,
        &Expression::FieldAccess {
            object: Box::new(Expression::Param { index: 0 }),
            class_name: "Test".to_string(),
            field_name: "x".to_string(),
            field_type: WasmFieldType::I32,
        },
    );

    match substituted {
        Expression::Add { left, .. } => match *left {
            Expression::FieldAccess { field_name, .. } => {
                assert_eq!(field_name, "x");
            }
            _ => panic!("Expected FieldAccess after substitution"),
        },
        _ => panic!("Expected Add"),
    }
}

#[test]
fn test_substitute_param_preserves_other_indices() {
    let expr = Expression::Add {
        left: Box::new(Expression::Param { index: 0 }),
        right: Box::new(Expression::Param { index: 1 }),
    };

    let substituted = substitute_param(expr, 0, &Expression::IntLiteral { value: 42 });

    match substituted {
        Expression::Add { left, right } => {
            assert!(matches!(*left, Expression::IntLiteral { value: 42 }));
            assert!(matches!(*right, Expression::Param { index: 1 }));
        }
        _ => panic!("Expected Add"),
    }
}
